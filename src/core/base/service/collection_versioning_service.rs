/*
Collection Versioning Service

The Collection Versioning Service manages versioning for Collection entities.
Collections are containers that hold multiple items, and this service tracks
changes to the collection structure, item additions/removals, and metadata changes.

This provides versioning similar to how content lists or taxonomies are versioned
in content management systems, where the collection structure and membership
are tracked over time.
*/

use crate::core::base::entity::collection::{CollectionType, Item};
use crate::core::base::service::node_version_service::{VersioningError, ChangeType, HashAlgorithm};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

// Add required dependencies for hashing
use sha2::{Sha256, Digest};
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CollectionVersion<T> {
    pub version_id: Uuid,
    pub collection_id: Uuid,
    pub version_number: u32,
    pub items: Vec<Item<T>>,
    pub metadata: CollectionVersionMetadata,
    pub created_at: DateTime<Utc>,
    pub created_by: Option<String>,
    pub change_summary: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CollectionVersionMetadata {
    pub is_current: bool,
    pub change_type: ChangeType,
    pub item_count: u32,
    pub structure_hash: String,
    pub content_hash: String,
    pub total_size_bytes: Option<u64>,
    pub collection_type: String,
}

#[derive(Debug, Clone)]
pub struct CollectionVersionHistory<T> {
    pub collection_id: Uuid,
    pub current_version: u32,
    pub versions: Vec<CollectionVersion<T>>,
    pub total_versions: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CollectionChangeType {
    Created,
    ItemAdded(Uuid),
    ItemRemoved(Uuid),
    ItemModified(Uuid),
    ItemsReordered,
    MetadataChanged,
    Bulk(Vec<CollectionChangeType>),
    Restored,
}

/// Collection Version Repository trait
pub trait CollectionVersionRepository<T> {
    fn save_collection_version(&self, version: &CollectionVersion<T>) -> Result<(), VersioningError>;
    fn get_collection_version(&self, collection_id: Uuid, version_number: u32) -> Result<Option<CollectionVersion<T>>, VersioningError>;
    fn get_current_collection_version(&self, collection_id: Uuid) -> Result<Option<CollectionVersion<T>>, VersioningError>;
    fn get_collection_version_history(&self, collection_id: Uuid) -> Result<CollectionVersionHistory<T>, VersioningError>;
    fn delete_collection_version(&self, collection_id: Uuid, version_number: u32) -> Result<(), VersioningError>;
    fn purge_old_collection_versions(&self, collection_id: Uuid, keep_count: u32) -> Result<u32, VersioningError>;
}

/// Collection Version Service
pub struct CollectionVersionService<T> {
    repository: Arc<dyn CollectionVersionRepository<T> + Send + Sync>,
    config: CollectionVersioningConfig,
}

#[derive(Debug, Clone)]
pub struct CollectionVersioningConfig {
    pub max_versions_per_collection: Option<u32>,
    pub auto_purge_enabled: bool,
    pub track_item_changes: bool,
    pub track_structure_changes: bool,
    pub hash_algorithm: HashAlgorithm,
    pub compress_large_collections: bool,
    pub max_items_per_version: Option<u32>,
}

impl Default for CollectionVersioningConfig {
    fn default() -> Self {
        Self {
            max_versions_per_collection: Some(30),
            auto_purge_enabled: true,
            track_item_changes: true,
            track_structure_changes: true,
            hash_algorithm: HashAlgorithm::Sha256,
            compress_large_collections: false,
            max_items_per_version: Some(1000),
        }
    }
}

impl<T> CollectionVersionService<T>
where
    T: Clone + serde::Serialize + for<'de> serde::Deserialize<'de> + PartialEq,
{
    pub fn new(
        repository: Arc<dyn CollectionVersionRepository<T> + Send + Sync>,
        config: Option<CollectionVersioningConfig>,
    ) -> Self {
        Self {
            repository,
            config: config.unwrap_or_default(),
        }
    }

    /// Create a new version when a collection is created or modified
    pub fn create_collection_version(
        &self,
        collection: &CollectionType<T>,
        change_type: ChangeType,
        created_by: Option<String>,
        change_summary: Option<String>,
    ) -> Result<CollectionVersion<T>, VersioningError> {
        // Check collection size limits
        if let Some(max_items) = self.config.max_items_per_version {
            if collection.items().len() > max_items as usize {
                return Err(VersioningError::StorageError(
                    format!("Collection exceeds maximum items per version: {} > {}", collection.items().len(), max_items)
                ));
            }
        }

        // Get current version number
        let current_version_number = match self.repository.get_current_collection_version(collection.uuid)? {
            Some(current) => current.version_number + 1,
            None => 1,
        };

        // Generate hashes
        let structure_hash = self.generate_structure_hash(collection.items())?;
        let content_hash = self.generate_content_hash(collection.items())?;

        // Calculate total size
        let total_size_bytes = if self.config.track_item_changes {
            Some(self.calculate_collection_size(collection.items())?)
        } else {
            None
        };

        let metadata = CollectionVersionMetadata {
            is_current: true,
            change_type,
            item_count: collection.items().len() as u32,
            structure_hash,
            content_hash,
            total_size_bytes,
            collection_type: std::any::type_name::<T>().to_string(),
        };

        let version = CollectionVersion {
            version_id: Uuid::new_v4(),
            collection_id: collection.uuid,
            version_number: current_version_number,
            items: collection.items().to_vec(),
            metadata,
            created_at: Utc::now(),
            created_by,
            change_summary,
        };

        // Mark previous version as not current
        if let Some(mut current) = self.repository.get_current_collection_version(collection.uuid)? {
            current.metadata.is_current = false;
            self.repository.save_collection_version(&current)?;
        }

        // Save new version
        self.repository.save_collection_version(&version)?;

        // Auto-purge if enabled
        if self.config.auto_purge_enabled {
            if let Some(max_versions) = self.config.max_versions_per_collection {
                if current_version_number > max_versions {
                    let _ = self.repository.purge_old_collection_versions(collection.uuid, max_versions);
                }
            }
        }

        Ok(version)
    }

    /// Get a specific version of a collection
    pub fn get_collection_version(&self, collection_id: Uuid, version_number: u32) -> Result<Option<CollectionVersion<T>>, VersioningError> {
        self.repository.get_collection_version(collection_id, version_number)
    }

    /// Get the current (latest) version of a collection
    pub fn get_current_collection_version(&self, collection_id: Uuid) -> Result<Option<CollectionVersion<T>>, VersioningError> {
        self.repository.get_current_collection_version(collection_id)
    }

    /// Get complete version history for a collection
    pub fn get_collection_version_history(&self, collection_id: Uuid) -> Result<CollectionVersionHistory<T>, VersioningError> {
        self.repository.get_collection_version_history(collection_id)
    }

    /// Compare two collection versions
    pub fn compare_collection_versions(
        &self,
        collection_id: Uuid,
        version1: u32,
        version2: u32,
    ) -> Result<CollectionVersionComparison<T>, VersioningError> {
        let v1 = self.repository.get_collection_version(collection_id, version1)?
            .ok_or_else(|| VersioningError::VersionNotFound(format!("{}:{}", collection_id, version1)))?;
        
        let v2 = self.repository.get_collection_version(collection_id, version2)?
            .ok_or_else(|| VersioningError::VersionNotFound(format!("{}:{}", collection_id, version2)))?;

        let changes = self.detect_collection_changes(&v1.items, &v2.items);

        let time_diff = v2.created_at.signed_duration_since(v1.created_at);

        // Extract comparison values before moving v1 and v2
        let structure_changed = v1.metadata.structure_hash != v2.metadata.structure_hash;
        let content_changed = v1.metadata.content_hash != v2.metadata.content_hash;
        let item_count_changed = v1.metadata.item_count != v2.metadata.item_count;
        let size_changed = v1.metadata.total_size_bytes != v2.metadata.total_size_bytes;

        Ok(CollectionVersionComparison {
            collection_id,
            version1: v1,
            version2: v2,
            structure_changed,
            content_changed,
            item_count_changed,
            size_changed,
            time_diff,
            detected_changes: changes,
        })
    }

    /// Restore a specific collection version
    pub fn restore_collection_version(
        &self,
        collection_id: Uuid,
        version_number: u32,
        restored_by: Option<String>,
    ) -> Result<CollectionVersion<T>, VersioningError> {
        let version_to_restore = self.repository.get_collection_version(collection_id, version_number)?
            .ok_or_else(|| VersioningError::VersionNotFound(format!("{}:{}", collection_id, version_number)))?;

        let collection = CollectionType::new(collection_id, version_to_restore.items);

        self.create_collection_version(
            &collection,
            ChangeType::Restored,
            restored_by,
            Some(format!("Restored from version {}", version_number)),
        )
    }

    /// Track specific item changes in a collection
    pub fn track_item_change(
        &self,
        collection: &CollectionType<T>,
        change_type: CollectionChangeType,
        created_by: Option<String>,
    ) -> Result<CollectionVersion<T>, VersioningError> {
        let change_summary = match &change_type {
            CollectionChangeType::ItemAdded(id) => Some(format!("Added item: {}", id)),
            CollectionChangeType::ItemRemoved(id) => Some(format!("Removed item: {}", id)),
            CollectionChangeType::ItemModified(id) => Some(format!("Modified item: {}", id)),
            CollectionChangeType::ItemsReordered => Some("Items reordered".to_string()),
            CollectionChangeType::MetadataChanged => Some("Metadata changed".to_string()),
            CollectionChangeType::Bulk(changes) => Some(format!("Bulk changes: {} operations", changes.len())),
            _ => None,
        };

        self.create_collection_version(
            collection,
            ChangeType::Updated,
            created_by,
            change_summary,
        )
    }

    fn generate_structure_hash(&self, items: &[Item<T>]) -> Result<String, VersioningError> {
        // Hash based on item count and order (structure only)
        let structure_data = items.len().to_string();
        self.hash_data(structure_data.as_bytes())
    }

    fn generate_content_hash(&self, items: &[Item<T>]) -> Result<String, VersioningError> {
        let serialized = serde_json::to_vec(items)
            .map_err(|e| VersioningError::SerializationError(e.to_string()))?;
        self.hash_data(&serialized)
    }

    fn hash_data(&self, data: &[u8]) -> Result<String, VersioningError> {
        match self.config.hash_algorithm {
            HashAlgorithm::Sha256 => {
                let hash = Sha256::digest(data);
                Ok(format!("{:x}", hash))
            },
            HashAlgorithm::Blake3 => {
                let hash = blake3::hash(data);
                Ok(hash.to_hex().to_string())
            },
            HashAlgorithm::Xxhash => {
                use std::collections::hash_map::DefaultHasher;
                let mut hasher = DefaultHasher::new();
                data.hash(&mut hasher);
                Ok(format!("{:x}", hasher.finish()))
            },
        }
    }

    fn calculate_collection_size(&self, items: &[Item<T>]) -> Result<u64, VersioningError> {
        let serialized = serde_json::to_vec(items)
            .map_err(|e| VersioningError::SerializationError(e.to_string()))?;
        Ok(serialized.len() as u64)
    }

    fn detect_collection_changes(&self, items1: &[Item<T>], items2: &[Item<T>]) -> Vec<CollectionChangeType> {
        let mut changes = Vec::new();

        if items1.len() != items2.len() {
            if items1.len() < items2.len() {
                changes.push(CollectionChangeType::ItemAdded(Uuid::new_v4())); // Simplified
            } else {
                changes.push(CollectionChangeType::ItemRemoved(Uuid::new_v4())); // Simplified
            }
        }

        // In a real implementation, you'd do more sophisticated diff analysis
        if items1.len() == items2.len() && items1 != items2 {
            changes.push(CollectionChangeType::ItemsReordered);
        }

        changes
    }
}

#[derive(Debug, Clone)]
pub struct CollectionVersionComparison<T> {
    pub collection_id: Uuid,
    pub version1: CollectionVersion<T>,
    pub version2: CollectionVersion<T>,
    pub structure_changed: bool,
    pub content_changed: bool,
    pub item_count_changed: bool,
    pub size_changed: bool,
    pub time_diff: chrono::Duration,
    pub detected_changes: Vec<CollectionChangeType>,
}

/// In-memory implementation for testing and development
pub struct InMemoryCollectionVersionRepository<T> {
    versions: Arc<RwLock<HashMap<(Uuid, u32), CollectionVersion<T>>>>,
    current_versions: Arc<RwLock<HashMap<Uuid, u32>>>,
}

impl<T> InMemoryCollectionVersionRepository<T> {
    pub fn new() -> Self {
        Self {
            versions: Arc::new(RwLock::new(HashMap::new())),
            current_versions: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl<T> CollectionVersionRepository<T> for InMemoryCollectionVersionRepository<T>
where
    T: Clone,
{
    fn save_collection_version(&self, version: &CollectionVersion<T>) -> Result<(), VersioningError> {
        let mut versions = self.versions.write()
            .map_err(|e| VersioningError::StorageError(e.to_string()))?;
        let mut current_versions = self.current_versions.write()
            .map_err(|e| VersioningError::StorageError(e.to_string()))?;

        versions.insert((version.collection_id, version.version_number), version.clone());
        
        if version.metadata.is_current {
            current_versions.insert(version.collection_id, version.version_number);
        }

        Ok(())
    }

    fn get_collection_version(&self, collection_id: Uuid, version_number: u32) -> Result<Option<CollectionVersion<T>>, VersioningError> {
        let versions = self.versions.read()
            .map_err(|e| VersioningError::StorageError(e.to_string()))?;
        Ok(versions.get(&(collection_id, version_number)).cloned())
    }

    fn get_current_collection_version(&self, collection_id: Uuid) -> Result<Option<CollectionVersion<T>>, VersioningError> {
        let current_versions = self.current_versions.read()
            .map_err(|e| VersioningError::StorageError(e.to_string()))?;
        let versions = self.versions.read()
            .map_err(|e| VersioningError::StorageError(e.to_string()))?;

        if let Some(&current_version_number) = current_versions.get(&collection_id) {
            Ok(versions.get(&(collection_id, current_version_number)).cloned())
        } else {
            Ok(None)
        }
    }

    fn get_collection_version_history(&self, collection_id: Uuid) -> Result<CollectionVersionHistory<T>, VersioningError> {
        let versions = self.versions.read()
            .map_err(|e| VersioningError::StorageError(e.to_string()))?;
        let current_versions = self.current_versions.read()
            .map_err(|e| VersioningError::StorageError(e.to_string()))?;

        let mut collection_versions: Vec<CollectionVersion<T>> = versions
            .iter()
            .filter(|((id, _), _)| *id == collection_id)
            .map(|(_, version)| version.clone())
            .collect();

        collection_versions.sort_by(|a, b| a.version_number.cmp(&b.version_number));

        let current_version = current_versions.get(&collection_id).copied().unwrap_or(0);
        let total_versions = collection_versions.len() as u32;

        Ok(CollectionVersionHistory {
            collection_id,
            current_version,
            versions: collection_versions,
            total_versions,
        })
    }

    fn delete_collection_version(&self, collection_id: Uuid, version_number: u32) -> Result<(), VersioningError> {
        let mut versions = self.versions.write()
            .map_err(|e| VersioningError::StorageError(e.to_string()))?;
        
        versions.remove(&(collection_id, version_number));
        Ok(())
    }

    fn purge_old_collection_versions(&self, collection_id: Uuid, keep_count: u32) -> Result<u32, VersioningError> {
        let history = self.get_collection_version_history(collection_id)?;
        
        if history.total_versions <= keep_count {
            return Ok(0);
        }

        let versions_to_delete = history.total_versions - keep_count;
        let mut deleted_count = 0;

        for version in history.versions.iter().take(versions_to_delete as usize) {
            self.delete_collection_version(collection_id, version.version_number)?;
            deleted_count += 1;
        }

        Ok(deleted_count)
    }
}

// ...existing code...

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct TestItem {
        name: String,
        value: i32,
    }

    #[test]
    fn test_collection_version_creation() {
        let repository = Arc::new(InMemoryCollectionVersionRepository::<TestItem>::new());
        let service = CollectionVersionService::new(repository, None);

        let items = vec![
            Item { item: TestItem { name: "item1".to_string(), value: 1 } },
            Item { item: TestItem { name: "item2".to_string(), value: 2 } },
        ];

        let collection = CollectionType {
            uuid: Uuid::new_v4(),
            items,
        };

        let version = service.create_collection_version(
            &collection,
            ChangeType::Created,
            Some("test_user".to_string()),
            Some("Initial collection creation".to_string()),
        ).unwrap();

        assert_eq!(version.version_number, 1);
        assert_eq!(version.collection_id, collection.uuid);
        assert_eq!(version.metadata.item_count, 2);
        assert!(version.metadata.is_current);
    }

    #[test]
    fn test_collection_version_restore() {
        let repository = Arc::new(InMemoryCollectionVersionRepository::<TestItem>::new());
        let service = CollectionVersionService::new(repository, None);

        let mut collection = CollectionType {
            uuid: Uuid::new_v4(),
            items: vec![Item { item: TestItem { name: "original".to_string(), value: 1 } }],
        };

        // Create initial version
        service.create_collection_version(&collection, ChangeType::Created, None, None).unwrap();

        // Modify collection
        collection.items.push(Item { item: TestItem { name: "added".to_string(), value: 2 } });
        service.create_collection_version(&collection, ChangeType::Updated, None, None).unwrap();

        // Restore to version 1
        let restored = service.restore_collection_version(
            collection.uuid, 
            1, 
            Some("test_user".to_string())
        ).unwrap();

        assert_eq!(restored.version_number, 3);
        assert_eq!(restored.metadata.change_type, ChangeType::Restored);
        assert_eq!(restored.items.len(), 1);
        assert_eq!(restored.items[0].item.name, "original");
    }

    #[test]
    fn test_collection_version_comparison() {
        let repository = Arc::new(InMemoryCollectionVersionRepository::<TestItem>::new());
        let service = CollectionVersionService::new(repository, None);

        let mut collection = CollectionType {
            uuid: Uuid::new_v4(),
            items: vec![Item { item: TestItem { name: "item1".to_string(), value: 1 } }],
        };

        // Create versions
        service.create_collection_version(&collection, ChangeType::Created, None, None).unwrap();
        
        collection.items.push(Item { item: TestItem { name: "item2".to_string(), value: 2 } });
        service.create_collection_version(&collection, ChangeType::Updated, None, None).unwrap();

        let comparison = service.compare_collection_versions(collection.uuid, 1, 2).unwrap();
        
        assert!(comparison.structure_changed);
        assert!(comparison.content_changed);
        assert!(comparison.item_count_changed);
        assert_eq!(comparison.version1.metadata.item_count, 1);
        assert_eq!(comparison.version2.metadata.item_count, 2);
    }

    #[test]
    fn test_track_item_change() {
        let repository = Arc::new(InMemoryCollectionVersionRepository::<TestItem>::new());
        let service = CollectionVersionService::new(repository, None);

        let collection = CollectionType {
            uuid: Uuid::new_v4(),
            items: vec![
                Item { item: TestItem { name: "item1".to_string(), value: 1 } },
                Item { item: TestItem { name: "item2".to_string(), value: 2 } },
            ],
        };

        let version = service.track_item_change(
            &collection,
            CollectionChangeType::ItemAdded(Uuid::new_v4()),
            Some("test_user".to_string()),
        ).unwrap();

        assert_eq!(version.version_number, 1);
        assert!(version.change_summary.as_ref().unwrap().contains("Added item"));
        assert_eq!(version.metadata.item_count, 2);
    }

    #[test]
    fn test_collection_size_limit() {
        let repository = Arc::new(InMemoryCollectionVersionRepository::<TestItem>::new());
        let config = CollectionVersioningConfig {
            max_items_per_version: Some(2),
            ..Default::default()
        };
        let service = CollectionVersionService::new(repository, Some(config));

        let collection = CollectionType {
            uuid: Uuid::new_v4(),
            items: vec![
                Item { item: TestItem { name: "item1".to_string(), value: 1 } },
                Item { item: TestItem { name: "item2".to_string(), value: 2 } },
                Item { item: TestItem { name: "item3".to_string(), value: 3 } },
            ],
        };

        let result = service.create_collection_version(
            &collection,
            ChangeType::Created,
            None,
            None,
        );

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), VersioningError::StorageError(_)));
    }

    #[test]
    fn test_auto_purge_versions() {
        let repository = Arc::new(InMemoryCollectionVersionRepository::<TestItem>::new());
        let config = CollectionVersioningConfig {
            max_versions_per_collection: Some(2),
            auto_purge_enabled: true,
            ..Default::default()
        };
        let service = CollectionVersionService::new(repository.clone(), Some(config));

        let mut collection = CollectionType {
            uuid: Uuid::new_v4(),
            items: vec![Item { item: TestItem { name: "item1".to_string(), value: 1 } }],
        };

        // Create 3 versions (should trigger auto-purge)
        service.create_collection_version(&collection, ChangeType::Created, None, None).unwrap();
        
        collection.items[0].item.value = 2;
        service.create_collection_version(&collection, ChangeType::Updated, None, None).unwrap();
        
        collection.items[0].item.value = 3;
        service.create_collection_version(&collection, ChangeType::Updated, None, None).unwrap();

        let history = service.get_collection_version_history(collection.uuid).unwrap();
        // Should only have 2 versions due to auto-purge
        assert_eq!(history.total_versions, 2);
    }

    #[test]
    fn test_bulk_changes() {
        let repository = Arc::new(InMemoryCollectionVersionRepository::<TestItem>::new());
        let service = CollectionVersionService::new(repository, None);

        let collection = CollectionType {
            uuid: Uuid::new_v4(),
            items: vec![Item { item: TestItem { name: "item1".to_string(), value: 1 } }],
        };

        let bulk_changes = vec![
            CollectionChangeType::ItemAdded(Uuid::new_v4()),
            CollectionChangeType::ItemModified(Uuid::new_v4()),
        ];

        let version = service.track_item_change(
            &collection,
            CollectionChangeType::Bulk(bulk_changes),
            Some("test_user".to_string()),
        ).unwrap();

        assert_eq!(version.version_number, 1);
        assert!(version.change_summary.as_ref().unwrap().contains("Bulk changes: 2 operations"));
    }

    #[test]
    fn test_hash_generation() {
        let repository = Arc::new(InMemoryCollectionVersionRepository::<TestItem>::new());
        let service = CollectionVersionService::new(repository, None);

        let collection1 = CollectionType {
            uuid: Uuid::new_v4(),
            items: vec![Item { item: TestItem { name: "item1".to_string(), value: 1 } }],
        };

        let collection2 = CollectionType {
            uuid: Uuid::new_v4(),
            items: vec![Item { item: TestItem { name: "item1".to_string(), value: 2 } }],
        };

        let version1 = service.create_collection_version(&collection1, ChangeType::Created, None, None).unwrap();
        let version2 = service.create_collection_version(&collection2, ChangeType::Created, None, None).unwrap();

        // Same structure but different content should have same structure hash but different content hash
        assert_eq!(version1.metadata.structure_hash, version2.metadata.structure_hash);
        assert_ne!(version1.metadata.content_hash, version2.metadata.content_hash);
    }

    #[test]
    fn test_versioning_config_defaults() {
        let config = CollectionVersioningConfig::default();
        
        assert_eq!(config.max_versions_per_collection, Some(30));
        assert_eq!(config.auto_purge_enabled, true);
        assert_eq!(config.track_item_changes, true);
        assert_eq!(config.track_structure_changes, true);
        assert_eq!(config.max_items_per_version, Some(1000));
        assert!(!config.compress_large_collections);
    }

    #[test]
    fn test_get_nonexistent_version() {
        let repository = Arc::new(InMemoryCollectionVersionRepository::<TestItem>::new());
        let service = CollectionVersionService::new(repository, None);

        let result = service.get_collection_version(Uuid::new_v4(), 1);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_get_current_version_empty_collection() {
        let repository = Arc::new(InMemoryCollectionVersionRepository::<TestItem>::new());
        let service = CollectionVersionService::new(repository, None);

        let result = service.get_current_collection_version(Uuid::new_v4());
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_restore_nonexistent_version() {
        let repository = Arc::new(InMemoryCollectionVersionRepository::<TestItem>::new());
        let service = CollectionVersionService::new(repository, None);

        let result = service.restore_collection_version(
            Uuid::new_v4(),
            1,
            Some("test_user".to_string()),
        );

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), VersioningError::VersionNotFound(_)));
    }

    #[test]
    fn test_compare_nonexistent_versions() {
        let repository = Arc::new(InMemoryCollectionVersionRepository::<TestItem>::new());
        let service = CollectionVersionService::new(repository, None);

        let result = service.compare_collection_versions(Uuid::new_v4(), 1, 2);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), VersioningError::VersionNotFound(_)));
    }

    #[test]
    fn test_different_hash_algorithms() {
        let repository1 = Arc::new(InMemoryCollectionVersionRepository::<TestItem>::new());
        let config1 = CollectionVersioningConfig {
            hash_algorithm: HashAlgorithm::Sha256,
            ..Default::default()
        };
        let service1 = CollectionVersionService::new(repository1, Some(config1));

        let repository2 = Arc::new(InMemoryCollectionVersionRepository::<TestItem>::new());
        let config2 = CollectionVersioningConfig {
            hash_algorithm: HashAlgorithm::Blake3,
            ..Default::default()
        };
        let service2 = CollectionVersionService::new(repository2, Some(config2));

        let collection = CollectionType {
            uuid: Uuid::new_v4(),
            items: vec![Item { item: TestItem { name: "item1".to_string(), value: 1 } }],
        };

        let version1 = service1.create_collection_version(&collection, ChangeType::Created, None, None).unwrap();
        let version2 = service2.create_collection_version(&collection, ChangeType::Created, None, None).unwrap();

        // Different hash algorithms should produce different hashes for the same content
        assert_ne!(version1.metadata.content_hash, version2.metadata.content_hash);
    }

    #[test]
    fn test_version_metadata_fields() {
        let repository = Arc::new(InMemoryCollectionVersionRepository::<TestItem>::new());
        let service = CollectionVersionService::new(repository, None);

        let collection = CollectionType {
            uuid: Uuid::new_v4(),
            items: vec![Item { item: TestItem { name: "item1".to_string(), value: 1 } }],
        };

        let version = service.create_collection_version(
            &collection,
            ChangeType::Created,
            Some("test_user".to_string()),
            Some("Test creation".to_string()),
        ).unwrap();

        assert!(version.metadata.is_current);
        assert_eq!(version.metadata.change_type, ChangeType::Created);
        assert_eq!(version.metadata.item_count, 1);
        assert!(!version.metadata.structure_hash.is_empty());
        assert!(!version.metadata.content_hash.is_empty());
        assert!(version.metadata.total_size_bytes.is_some());
        assert_eq!(version.created_by, Some("test_user".to_string()));
        assert_eq!(version.change_summary, Some("Test creation".to_string()));
    }

    #[test]
    fn test_collection_version_history() {
        let repository = Arc::new(InMemoryCollectionVersionRepository::<TestItem>::new());
        let service = CollectionVersionService::new(repository, None);

        let mut collection = CollectionType {
            uuid: Uuid::new_v4(),
            items: vec![Item { item: TestItem { name: "item1".to_string(), value: 1 } }],
        };

        // Create initial version
        service.create_collection_version(&collection, ChangeType::Created, None, None).unwrap();

        // Add an item and create new version
        collection.items.push(Item { item: TestItem { name: "item2".to_string(), value: 2 } });
        service.create_collection_version(&collection, ChangeType::Updated, None, None).unwrap();

        let history = service.get_collection_version_history(collection.uuid).unwrap();
        assert_eq!(history.total_versions, 2);
        assert_eq!(history.current_version, 2);
        assert_eq!(history.versions[0].metadata.item_count, 1);
        assert_eq!(history.versions[1].metadata.item_count, 2);
    }
}