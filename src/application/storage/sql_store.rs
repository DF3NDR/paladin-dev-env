/*
SQL Database Application Repository

An application repository is a port that defines the operations that the application layer can perform on the data layer (infrastructure repositories) and the interface requirements that the data layers adapter must implement. The application repository is implemented by the data layer, which contains the actual implementation of the operations defined in the application repository. The application repository acts as a bridge between the application layer and the data layer, allowing the application layer to interact with the data layer without having to know the details of the data layer implementation.
*/

use crate::core::platform::container::content::{ContentItem, ContentItemError};
use crate::core::platform::container::content_list::{ContentList, ContentItemToFetch, CreateContentListError};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use thiserror::Error;
use std::collections::HashMap;

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
    ContentListError(#[from] CreateContentListError),
}

/// Content Repository
/// Defines operations for managing ContentItem entities
pub trait ContentRepository {
    /// Get content item by hash
    fn get_by_hash(&self, hash: &str) -> Result<Option<ContentItem>, RepositoryError>;
    
    /// Get content item by UUID
    fn get_by_id(&self, id: Uuid) -> Result<Option<ContentItem>, RepositoryError>;
    
    /// Save a content item
    fn save(&self, content: &ContentItem) -> Result<(), RepositoryError>;
    
    /// Update an existing content item
    fn update(&self, content: &ContentItem) -> Result<(), RepositoryError>;
    
    /// Delete a content item by UUID
    fn delete(&self, id: Uuid) -> Result<(), RepositoryError>;
    
    /// Find content items by tags
    fn find_by_tags(&self, tags: &[String]) -> Result<Vec<ContentItem>, RepositoryError>;
    
    /// Find content items by source
    fn find_by_source(&self, source: &str) -> Result<Vec<ContentItem>, RepositoryError>;
    
    /// Find content items created within a date range
    fn find_by_date_range(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> Result<Vec<ContentItem>, RepositoryError>;
    
    /// Get all content items with pagination
    fn find_all(&self, limit: Option<u32>, offset: Option<u32>) -> Result<Vec<ContentItem>, RepositoryError>;
    
    /// Count total content items
    fn count(&self) -> Result<u64, RepositoryError>;
    
    /// Check if content with hash exists
    fn exists_by_hash(&self, hash: &str) -> Result<bool, RepositoryError>;
}

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

/// Content Item To Fetch Repository
/// Manages items that need to be fetched
pub trait ContentItemToFetchRepository {
    /// Get content item to fetch by UUID
    fn get_by_id(&self, id: Uuid) -> Result<Option<ContentItemToFetch>, RepositoryError>;
    
    /// Save a content item to fetch
    fn save(&self, item_to_fetch: &ContentItemToFetch) -> Result<(), RepositoryError>;
    
    /// Delete a content item to fetch by UUID
    fn delete(&self, id: Uuid) -> Result<(), RepositoryError>;
    
    /// Get all pending items to fetch
    fn get_pending(&self, limit: Option<u32>) -> Result<Vec<ContentItemToFetch>, RepositoryError>;
    
    /// Mark item as fetched (move to content items)
    fn mark_as_fetched(&self, id: Uuid, content_item: &ContentItem) -> Result<(), RepositoryError>;
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
pub trait MigrationManager {
    /// Run pending migrations
    fn migrate(&self) -> Result<(), RepositoryError>;
    
    /// Check if migrations are up to date
    fn is_up_to_date(&self) -> Result<bool, RepositoryError>;
    
    /// Get current migration version
    fn current_version(&self) -> Result<Option<String>, RepositoryError>;
}

/// Combined Repository Interface
/// Aggregates all repository interfaces for convenience
pub trait SqlStore: ContentRepository + ContentListRepository + ContentItemToFetchRepository + TransactionManager + MigrationManager {
    /// Get repository statistics
    fn get_stats(&self) -> Result<RepositoryStats, RepositoryError>;
    
    /// Perform database health check
    fn health_check(&self) -> Result<bool, RepositoryError>;
    
    /// Clean up old or orphaned records
    fn cleanup(&self, older_than: DateTime<Utc>) -> Result<u64, RepositoryError>;
}

#[derive(Debug, Clone, PartialEq)]
pub struct RepositoryStats {
    pub total_content_items: u64,
    pub total_content_lists: u64,
    pub total_items_to_fetch: u64,
    pub database_size_bytes: Option<u64>,
    pub last_updated: DateTime<Utc>,
}