//! Entity Version Service.
//!
//! This module keeps revision history for base entities. [`EntityVersionService`] creates a new
//! [`EntityVersion`] each time a caller records a creation or modification, stores the
//! historical versions through an [`EntityVersionRepository`], and offers retrieval, restore,
//! comparison and purge operations over that history. It implements a revision system similar
//! to Drupal's node revisions: each change adds a version while the complete history is kept.
//!
//! Only callers that route writes through [`EntityVersionService`] honour the base entity
//! wrapper's `version` flag. [`EntityVersionService::create_version`] returns
//! [`VersioningError::VersioningDisabled`] when that flag is false; nothing else in the
//! platform consults it.
//!
//! # Persisted records
//!
//! [`EntityVersion`] is serialized as `entity_id` and `entity_data`. Records written by v0.10
//! used the keys `node_id` and `node_data`; serde read-aliases keep those records loading
//! (ADR-0060). New writes only ever emit the new keys, and a record carrying both spellings of
//! one key is rejected as a duplicate field.

use crate::base::entity::Entity;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Clone, Error)]
pub enum VersioningError {
    #[error("Version not found: {0}")]
    VersionNotFound(String),
    #[error("Entity not found: {0}")]
    EntityNotFound(Uuid),
    #[error("Invalid version number: {0}")]
    InvalidVersionNumber(u32),
    #[error("Versioning disabled for entity: {0}")]
    VersioningDisabled(Uuid),
    #[error("Storage error: {0}")]
    StorageError(String),
    #[error("Serialization error: {0}")]
    SerializationError(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EntityVersion<T> {
    pub version_id: Uuid,
    /// Identifier of the versioned entity. v0.10 records spelled this key `node_id`.
    #[serde(alias = "node_id")]
    pub entity_id: Uuid,
    pub version_number: u32,
    /// Snapshot of the entity payload. v0.10 records spelled this key `node_data`.
    #[serde(alias = "node_data")]
    pub entity_data: T,
    pub metadata: EntityVersionMetadata,
    pub created_at: DateTime<Utc>,
    pub created_by: Option<String>,
    pub change_summary: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EntityVersionMetadata {
    pub is_published: bool,
    pub is_current: bool,
    pub tags: Vec<String>,
    pub change_type: ChangeType,
    pub content_hash: String,
    pub size_bytes: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ChangeType {
    Created,
    Updated,
    Published,
    Unpublished,
    Restored,
    Archived,
}

#[derive(Debug, Clone)]
pub struct VersionHistory<T> {
    pub entity_id: Uuid,
    pub current_version: u32,
    pub versions: Vec<EntityVersion<T>>,
    pub total_versions: u32,
}

/// Entity Version Repository trait
/// Defines the contract for persisting entity versions
pub trait EntityVersionRepository<T> {
    fn save_version(&self, version: &EntityVersion<T>) -> Result<(), VersioningError>;
    fn get_version(
        &self,
        entity_id: Uuid,
        version_number: u32,
    ) -> Result<Option<EntityVersion<T>>, VersioningError>;
    fn get_current_version(
        &self,
        entity_id: Uuid,
    ) -> Result<Option<EntityVersion<T>>, VersioningError>;
    fn get_version_history(&self, entity_id: Uuid) -> Result<VersionHistory<T>, VersioningError>;
    fn delete_version(&self, entity_id: Uuid, version_number: u32) -> Result<(), VersioningError>;
    fn purge_old_versions(&self, entity_id: Uuid, keep_count: u32) -> Result<u32, VersioningError>;
}

/// Entity Version Service
/// Core service for managing entity versioning
///
/// # Examples
///
/// ```
/// use paladin_core::base::service::entity_version_service::{
///     EntityVersion, EntityVersionRepository, EntityVersionService, VersioningError,
/// };
/// use std::sync::Arc;
/// use uuid::Uuid;
///
/// fn latest_version<T>(
///     repository: Arc<dyn EntityVersionRepository<T> + Send + Sync>,
///     entity_id: Uuid,
/// ) -> Result<Option<EntityVersion<T>>, VersioningError>
/// where
///     T: Clone + serde::Serialize + for<'de> serde::Deserialize<'de>,
/// {
///     let service = EntityVersionService::new(repository, None);
///     service.get_current_version(entity_id)
/// }
/// ```
pub struct EntityVersionService<T> {
    repository: Arc<dyn EntityVersionRepository<T> + Send + Sync>,
    config: VersioningConfig,
}

#[derive(Debug, Clone)]
pub struct VersioningConfig {
    pub max_versions_per_entity: Option<u32>,
    pub auto_purge_enabled: bool,
    pub compress_old_versions: bool,
    pub hash_algorithm: HashAlgorithm,
}

#[derive(Debug, Clone)]
pub enum HashAlgorithm {
    Sha256,
    Blake3,
    Xxhash,
}

impl Default for VersioningConfig {
    fn default() -> Self {
        Self {
            max_versions_per_entity: Some(50),
            auto_purge_enabled: true,
            compress_old_versions: false,
            hash_algorithm: HashAlgorithm::Sha256,
        }
    }
}

impl<T> EntityVersionService<T>
where
    T: Clone + serde::Serialize + for<'de> serde::Deserialize<'de>,
{
    pub fn new(
        repository: Arc<dyn EntityVersionRepository<T> + Send + Sync>,
        config: Option<VersioningConfig>,
    ) -> Self {
        Self {
            repository,
            config: config.unwrap_or_default(),
        }
    }

    /// Create a new version when an entity is created or modified
    pub fn create_version(
        &self,
        entity: &Entity<T>,
        change_type: ChangeType,
        created_by: Option<String>,
        change_summary: Option<String>,
    ) -> Result<EntityVersion<T>, VersioningError> {
        if !entity.version {
            return Err(VersioningError::VersioningDisabled(entity.uuid));
        }

        // Get current version number
        let current_version_number = match self.repository.get_current_version(entity.uuid)? {
            Some(current) => current.version_number + 1,
            None => 1,
        };

        // Generate content hash
        let content_hash = self.generate_content_hash(&entity.data)?;

        // Calculate serialized size
        let size_bytes = self.calculate_size(&entity.data)?;

        let metadata = EntityVersionMetadata {
            is_published: matches!(change_type, ChangeType::Created | ChangeType::Published),
            is_current: true,
            tags: Vec::new(),
            change_type,
            content_hash,
            size_bytes: Some(size_bytes),
        };

        let version = EntityVersion {
            version_id: Uuid::new_v4(),
            entity_id: entity.uuid,
            version_number: current_version_number,
            entity_data: entity.data.clone(),
            metadata,
            created_at: Utc::now(),
            created_by,
            change_summary,
        };

        // Mark previous version as not current
        if let Some(mut current) = self.repository.get_current_version(entity.uuid)? {
            current.metadata.is_current = false;
            self.repository.save_version(&current)?;
        }

        // Save new version
        self.repository.save_version(&version)?;

        // Auto-purge if enabled
        if self.config.auto_purge_enabled
            && let Some(max_versions) = self.config.max_versions_per_entity
            && current_version_number > max_versions
        {
            let _ = self
                .repository
                .purge_old_versions(entity.uuid, max_versions);
        }

        Ok(version)
    }

    /// Get a specific version of an entity
    pub fn get_version(
        &self,
        entity_id: Uuid,
        version_number: u32,
    ) -> Result<Option<EntityVersion<T>>, VersioningError> {
        self.repository.get_version(entity_id, version_number)
    }

    /// Get the current (latest) version of an entity
    pub fn get_current_version(
        &self,
        entity_id: Uuid,
    ) -> Result<Option<EntityVersion<T>>, VersioningError> {
        self.repository.get_current_version(entity_id)
    }

    /// Get complete version history for an entity
    pub fn get_version_history(
        &self,
        entity_id: Uuid,
    ) -> Result<VersionHistory<T>, VersioningError> {
        self.repository.get_version_history(entity_id)
    }

    /// Restore a specific version as the current version
    pub fn restore_version(
        &self,
        entity_id: Uuid,
        version_number: u32,
        restored_by: Option<String>,
    ) -> Result<EntityVersion<T>, VersioningError> {
        let version_to_restore = self
            .repository
            .get_version(entity_id, version_number)?
            .ok_or_else(|| {
                VersioningError::VersionNotFound(format!("{}:{}", entity_id, version_number))
            })?;

        // Create a new version based on the restored data
        let entity = Entity {
            uuid: entity_id,
            created: Utc::now(), // This would normally come from the original entity
            modified: Utc::now(),
            data: version_to_restore.entity_data,
            name: None, // This would normally come from the original entity
            version: true,
        };

        self.create_version(
            &entity,
            ChangeType::Restored,
            restored_by,
            Some(format!("Restored from version {}", version_number)),
        )
    }

    /// Compare two versions and return differences
    pub fn compare_versions(
        &self,
        entity_id: Uuid,
        version1: u32,
        version2: u32,
    ) -> Result<VersionComparison<T>, VersioningError> {
        let v1 = self
            .repository
            .get_version(entity_id, version1)?
            .ok_or_else(|| {
                VersioningError::VersionNotFound(format!("{}:{}", entity_id, version1))
            })?;

        let v2 = self
            .repository
            .get_version(entity_id, version2)?
            .ok_or_else(|| {
                VersioningError::VersionNotFound(format!("{}:{}", entity_id, version2))
            })?;

        let hash_changed = v1.metadata.content_hash != v2.metadata.content_hash;
        let size_changed = v1.metadata.size_bytes != v2.metadata.size_bytes;
        let time_diff = v2.created_at.signed_duration_since(v1.created_at);

        Ok(VersionComparison {
            entity_id,
            version1: v1,
            version2: v2,
            hash_changed,
            size_changed,
            time_diff,
        })
    }

    /// Purge old versions manually
    pub fn purge_old_versions(
        &self,
        entity_id: Uuid,
        keep_count: u32,
    ) -> Result<u32, VersioningError> {
        self.repository.purge_old_versions(entity_id, keep_count)
    }

    fn generate_content_hash(&self, content: &T) -> Result<String, VersioningError> {
        let serialized = serde_json::to_vec(content)
            .map_err(|e| VersioningError::SerializationError(e.to_string()))?;

        match self.config.hash_algorithm {
            HashAlgorithm::Sha256 => {
                use sha2::{Digest, Sha256};
                let hash = Sha256::digest(&serialized);
                Ok(format!("{:x}", hash))
            }
            HashAlgorithm::Blake3 => {
                let hash = blake3::hash(&serialized);
                Ok(hash.to_hex().to_string())
            }
            HashAlgorithm::Xxhash => {
                use std::collections::hash_map::DefaultHasher;
                use std::hash::{Hash, Hasher};
                let mut hasher = DefaultHasher::new();
                serialized.hash(&mut hasher);
                Ok(format!("{:x}", hasher.finish()))
            }
        }
    }

    fn calculate_size(&self, content: &T) -> Result<u64, VersioningError> {
        let serialized = serde_json::to_vec(content)
            .map_err(|e| VersioningError::SerializationError(e.to_string()))?;
        Ok(serialized.len() as u64)
    }
}

#[derive(Debug, Clone)]
pub struct VersionComparison<T> {
    pub entity_id: Uuid,
    pub version1: EntityVersion<T>,
    pub version2: EntityVersion<T>,
    pub hash_changed: bool,
    pub size_changed: bool,
    pub time_diff: chrono::Duration,
}

/// Type alias for entity version storage
type EntityVersionMap<T> = Arc<RwLock<HashMap<(Uuid, u32), EntityVersion<T>>>>;

/// In-memory implementation for testing and development
pub struct InMemoryEntityVersionRepository<T> {
    versions: EntityVersionMap<T>,
    current_versions: Arc<RwLock<HashMap<Uuid, u32>>>,
}

impl<T> Default for InMemoryEntityVersionRepository<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> InMemoryEntityVersionRepository<T> {
    pub fn new() -> Self {
        Self {
            versions: Arc::new(RwLock::new(HashMap::new())),
            current_versions: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl<T> EntityVersionRepository<T> for InMemoryEntityVersionRepository<T>
where
    T: Clone,
{
    fn save_version(&self, version: &EntityVersion<T>) -> Result<(), VersioningError> {
        let mut versions = self
            .versions
            .write()
            .map_err(|e| VersioningError::StorageError(e.to_string()))?;
        let mut current_versions = self
            .current_versions
            .write()
            .map_err(|e| VersioningError::StorageError(e.to_string()))?;

        versions.insert((version.entity_id, version.version_number), version.clone());

        if version.metadata.is_current {
            current_versions.insert(version.entity_id, version.version_number);
        }

        Ok(())
    }

    fn get_version(
        &self,
        entity_id: Uuid,
        version_number: u32,
    ) -> Result<Option<EntityVersion<T>>, VersioningError> {
        let versions = self
            .versions
            .read()
            .map_err(|e| VersioningError::StorageError(e.to_string()))?;
        Ok(versions.get(&(entity_id, version_number)).cloned())
    }

    fn get_current_version(
        &self,
        entity_id: Uuid,
    ) -> Result<Option<EntityVersion<T>>, VersioningError> {
        let current_versions = self
            .current_versions
            .read()
            .map_err(|e| VersioningError::StorageError(e.to_string()))?;
        let versions = self
            .versions
            .read()
            .map_err(|e| VersioningError::StorageError(e.to_string()))?;

        if let Some(&current_version_number) = current_versions.get(&entity_id) {
            Ok(versions.get(&(entity_id, current_version_number)).cloned())
        } else {
            Ok(None)
        }
    }

    fn get_version_history(&self, entity_id: Uuid) -> Result<VersionHistory<T>, VersioningError> {
        let versions = self
            .versions
            .read()
            .map_err(|e| VersioningError::StorageError(e.to_string()))?;
        let current_versions = self
            .current_versions
            .read()
            .map_err(|e| VersioningError::StorageError(e.to_string()))?;

        let mut entity_versions: Vec<EntityVersion<T>> = versions
            .iter()
            .filter(|((id, _), _)| *id == entity_id)
            .map(|(_, version)| version.clone())
            .collect();

        entity_versions.sort_by_key(|a| a.version_number);

        let current_version = current_versions.get(&entity_id).copied().unwrap_or(0);
        let total_versions = entity_versions.len() as u32;

        Ok(VersionHistory {
            entity_id,
            current_version,
            versions: entity_versions,
            total_versions,
        })
    }

    fn delete_version(&self, entity_id: Uuid, version_number: u32) -> Result<(), VersioningError> {
        let mut versions = self
            .versions
            .write()
            .map_err(|e| VersioningError::StorageError(e.to_string()))?;

        versions.remove(&(entity_id, version_number));
        Ok(())
    }

    fn purge_old_versions(&self, entity_id: Uuid, keep_count: u32) -> Result<u32, VersioningError> {
        let history = self.get_version_history(entity_id)?;

        if history.total_versions <= keep_count {
            return Ok(0);
        }

        let versions_to_delete = history.total_versions - keep_count;
        let mut deleted_count = 0;

        // Delete oldest versions, keeping the most recent ones
        for version in history.versions.iter().take(versions_to_delete as usize) {
            self.delete_version(entity_id, version.version_number)?;
            deleted_count += 1;
        }

        Ok(deleted_count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct TestData {
        value: String,
        number: i32,
    }

    #[test]
    fn test_entity_version_creation() {
        let repository = Arc::new(InMemoryEntityVersionRepository::new());
        let service = EntityVersionService::new(repository, None);

        let test_data = TestData {
            value: "test".to_string(),
            number: 42,
        };

        let entity = Entity::new(test_data, Some("test_entity".to_string()));

        let version = service
            .create_version(
                &entity,
                ChangeType::Created,
                Some("test_user".to_string()),
                Some("Initial creation".to_string()),
            )
            .unwrap();

        assert_eq!(version.version_number, 1);
        assert_eq!(version.entity_id, entity.uuid);
        assert!(version.metadata.is_current);
        assert_eq!(version.metadata.change_type, ChangeType::Created);
    }

    #[test]
    fn test_version_history() {
        let repository = Arc::new(InMemoryEntityVersionRepository::new());
        let service = EntityVersionService::new(repository, None);

        let test_data = TestData {
            value: "test".to_string(),
            number: 42,
        };

        let entity = Entity::new(test_data.clone(), Some("test_entity".to_string()));

        // Create initial version
        service
            .create_version(&entity, ChangeType::Created, None, None)
            .unwrap();

        // Create updated version
        let updated_data = TestData {
            value: "updated".to_string(),
            number: 43,
        };
        let updated_entity = Entity {
            data: updated_data,
            ..entity
        };

        service
            .create_version(&updated_entity, ChangeType::Updated, None, None)
            .unwrap();

        let history = service.get_version_history(entity.uuid).unwrap();
        assert_eq!(history.total_versions, 2);
        assert_eq!(history.current_version, 2);
    }

    #[test]
    fn test_version_restore() {
        let repository = Arc::new(InMemoryEntityVersionRepository::new());
        let service = EntityVersionService::new(repository, None);

        let test_data = TestData {
            value: "original".to_string(),
            number: 1,
        };

        let entity = Entity::new(test_data.clone(), Some("test_entity".to_string()));

        // Create initial version
        service
            .create_version(&entity, ChangeType::Created, None, None)
            .unwrap();

        // Create updated version
        let updated_data = TestData {
            value: "updated".to_string(),
            number: 2,
        };
        let updated_entity = Entity {
            data: updated_data,
            ..entity
        };

        service
            .create_version(&updated_entity, ChangeType::Updated, None, None)
            .unwrap();

        // Restore original version
        let restored = service
            .restore_version(entity.uuid, 1, Some("test_user".to_string()))
            .unwrap();

        assert_eq!(restored.version_number, 3);
        assert_eq!(restored.metadata.change_type, ChangeType::Restored);
    }

    #[test]
    fn entity_version_rejects_both_key_spellings() -> Result<(), Box<dyn std::error::Error>> {
        let repository = Arc::new(InMemoryEntityVersionRepository::new());
        let service = EntityVersionService::new(repository, None);
        let entity = Entity::new(
            TestData {
                value: "test".to_string(),
                number: 42,
            },
            Some("test_entity".to_string()),
        );
        let version = service.create_version(&entity, ChangeType::Created, None, None)?;
        let value = serde_json::to_value(&version)?;

        // New writes emit only the new keys.
        let record = value
            .as_object()
            .ok_or("a version serializes to an object")?;
        assert!(record.contains_key("entity_id"));
        assert!(record.contains_key("entity_data"));
        assert!(!record.contains_key("node_id"));
        assert!(!record.contains_key("node_data"));

        // A record carrying both spellings of one key is rejected, not merged.
        for (old, new) in [("node_id", "entity_id"), ("node_data", "entity_data")] {
            let mut doubled = value.clone();
            let copy = doubled[new].clone();
            doubled
                .as_object_mut()
                .ok_or("a version serializes to an object")?
                .insert(old.to_owned(), copy);
            let text = serde_json::to_string(&doubled)?;
            let error = serde_json::from_str::<EntityVersion<TestData>>(&text)
                .err()
                .ok_or_else(|| format!("a record with both `{old}` and `{new}` was accepted"))?;
            assert!(
                error.to_string().contains("duplicate field"),
                "unexpected error for `{old}` beside `{new}`: {error}"
            );
        }
        Ok(())
    }
}
