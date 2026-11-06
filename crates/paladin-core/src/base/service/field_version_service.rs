/*
Field Version Service

The Field Version Service manages versioning for Field entities. Fields are child
entities of Entities and can have independent versioning enabled. This service tracks
changes to individual fields and maintains their history.

This allows for granular version control where only specific fields of an entity
are versioned, similar to Drupal's field-level revision system.
*/

use crate::base::entity::field::Field;
use crate::base::service::entity_version_service::{ChangeType, HashAlgorithm, VersioningError};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldVersion<T> {
    pub version_id: Uuid,
    pub field_id: Uuid,
    pub node_id: Uuid,
    pub version_number: u32,
    pub field_value: T,
    pub metadata: FieldVersionMetadata,
    pub created_at: DateTime<Utc>,
    pub created_by: Option<String>,
    pub change_summary: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldVersionMetadata {
    pub is_current: bool,
    pub change_type: ChangeType,
    pub content_hash: String,
    pub size_bytes: Option<u64>,
    pub validation_status: ValidationStatus,
    pub field_type: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ValidationStatus {
    Valid,
    Invalid(String),
    Pending,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct FieldVersionHistory<T> {
    pub field_id: Uuid,
    pub node_id: Uuid,
    pub current_version: u32,
    pub versions: Vec<FieldVersion<T>>,
    pub total_versions: u32,
}

/// Field Version Repository trait
pub trait FieldVersionRepository<T> {
    fn save_field_version(&self, version: &FieldVersion<T>) -> Result<(), VersioningError>;
    fn get_field_version(
        &self,
        field_id: Uuid,
        version_number: u32,
    ) -> Result<Option<FieldVersion<T>>, VersioningError>;
    fn get_current_field_version(
        &self,
        field_id: Uuid,
    ) -> Result<Option<FieldVersion<T>>, VersioningError>;
    /// Return the complete, version-ordered history of a field.
    ///
    /// # Errors
    ///
    /// [`VersioningError::EntityNotFound`] carrying `field_id` when the field
    /// has no stored version: there is no history and no parent id to report
    /// (FIXES-02, D-03). Storage failures surface as
    /// [`VersioningError::StorageError`].
    fn get_field_version_history(
        &self,
        field_id: Uuid,
    ) -> Result<FieldVersionHistory<T>, VersioningError>;
    fn get_node_field_versions(
        &self,
        node_id: Uuid,
    ) -> Result<Vec<FieldVersion<T>>, VersioningError>;
    fn delete_field_version(
        &self,
        field_id: Uuid,
        version_number: u32,
    ) -> Result<(), VersioningError>;
    fn purge_old_field_versions(
        &self,
        field_id: Uuid,
        keep_count: u32,
    ) -> Result<u32, VersioningError>;
}

/// Field Version Service
///
/// Unfinished scaffolding of the content / general-data orchestration system,
/// retained pending the ADR-0060 (g) review. The review is recorded in
/// `55-BASE-LAYER-REVIEW.md` (keep and fence). The fence is that a field with
/// no stored version is refused with [`VersioningError::EntityNotFound`] for
/// its history rather than given an invented parent id. The in-memory
/// repository is the only implementation.
///
/// # Examples
///
/// ```
/// use paladin_core::base::service::field_version_service::{
///     FieldVersion, FieldVersionRepository, FieldVersionService,
/// };
/// use paladin_core::base::service::entity_version_service::VersioningError;
/// use std::sync::Arc;
/// use uuid::Uuid;
///
/// fn latest_field_version<T>(
///     repository: Arc<dyn FieldVersionRepository<T> + Send + Sync>,
///     field_id: Uuid,
/// ) -> Result<Option<FieldVersion<T>>, VersioningError>
/// where
///     T: Clone + serde::Serialize + for<'de> serde::Deserialize<'de>,
/// {
///     let service = FieldVersionService::new(repository, None);
///     service.get_current_field_version(field_id)
/// }
/// ```
pub struct FieldVersionService<T> {
    repository: Arc<dyn FieldVersionRepository<T> + Send + Sync>,
    config: FieldVersioningConfig,
}

#[derive(Debug, Clone)]
pub struct FieldVersioningConfig {
    pub max_versions_per_field: Option<u32>,
    pub auto_purge_enabled: bool,
    pub validate_on_save: bool,
    pub hash_algorithm: HashAlgorithm,
    pub track_size_changes: bool,
}

impl Default for FieldVersioningConfig {
    fn default() -> Self {
        Self {
            max_versions_per_field: Some(25),
            auto_purge_enabled: true,
            validate_on_save: true,
            hash_algorithm: HashAlgorithm::Sha256,
            track_size_changes: true,
        }
    }
}

impl<T> FieldVersionService<T>
where
    T: Clone + serde::Serialize + for<'de> serde::Deserialize<'de>,
{
    pub fn new(
        repository: Arc<dyn FieldVersionRepository<T> + Send + Sync>,
        config: Option<FieldVersioningConfig>,
    ) -> Self {
        Self {
            repository,
            config: config.unwrap_or_default(),
        }
    }

    /// Create a new version when a field is created or modified
    pub fn create_field_version(
        &self,
        field: &Field<T>,
        change_type: ChangeType,
        created_by: Option<String>,
        change_summary: Option<String>,
    ) -> Result<FieldVersion<T>, VersioningError> {
        // Get current version number
        let current_version_number = match self.repository.get_current_field_version(field.fid)? {
            Some(current) => current.version_number + 1,
            None => 1,
        };

        // Generate content hash
        let content_hash = self.generate_content_hash(&field.value)?;

        // Calculate serialized size
        let size_bytes = if self.config.track_size_changes {
            Some(self.calculate_size(&field.value)?)
        } else {
            None
        };

        // Validate field if enabled
        let validation_status = if self.config.validate_on_save {
            self.validate_field_value(&field.value)
        } else {
            ValidationStatus::Unknown
        };

        let metadata = FieldVersionMetadata {
            is_current: true,
            change_type,
            content_hash,
            size_bytes,
            validation_status,
            field_type: std::any::type_name::<T>().to_string(),
        };

        let version = FieldVersion {
            version_id: Uuid::new_v4(),
            field_id: field.fid,
            node_id: field.nid,
            version_number: current_version_number,
            field_value: field.value.clone(),
            metadata,
            created_at: Utc::now(),
            created_by,
            change_summary,
        };

        // Mark previous version as not current
        if let Some(mut current) = self.repository.get_current_field_version(field.fid)? {
            current.metadata.is_current = false;
            self.repository.save_field_version(&current)?;
        }

        // Save new version
        self.repository.save_field_version(&version)?;

        // Auto-purge if enabled
        if self.config.auto_purge_enabled
            && let Some(max_versions) = self.config.max_versions_per_field
            && current_version_number > max_versions
        {
            let _ = self
                .repository
                .purge_old_field_versions(field.fid, max_versions);
        }

        Ok(version)
    }

    /// Get a specific version of a field
    pub fn get_field_version(
        &self,
        field_id: Uuid,
        version_number: u32,
    ) -> Result<Option<FieldVersion<T>>, VersioningError> {
        self.repository.get_field_version(field_id, version_number)
    }

    /// Get the current (latest) version of a field
    pub fn get_current_field_version(
        &self,
        field_id: Uuid,
    ) -> Result<Option<FieldVersion<T>>, VersioningError> {
        self.repository.get_current_field_version(field_id)
    }

    /// Get complete version history for a field
    ///
    /// # Errors
    ///
    /// [`VersioningError::EntityNotFound`] carrying `field_id` when the field
    /// has no stored version, because there is no history and no parent id to
    /// report (FIXES-02, D-03).
    pub fn get_field_version_history(
        &self,
        field_id: Uuid,
    ) -> Result<FieldVersionHistory<T>, VersioningError> {
        self.repository.get_field_version_history(field_id)
    }

    /// Get all field versions for a specific node
    pub fn get_node_field_versions(
        &self,
        node_id: Uuid,
    ) -> Result<Vec<FieldVersion<T>>, VersioningError> {
        self.repository.get_node_field_versions(node_id)
    }

    /// Compare two field versions
    pub fn compare_field_versions(
        &self,
        field_id: Uuid,
        version1: u32,
        version2: u32,
    ) -> Result<FieldVersionComparison<T>, VersioningError> {
        let v1 = self
            .repository
            .get_field_version(field_id, version1)?
            .ok_or_else(|| {
                VersioningError::VersionNotFound(format!("{}:{}", field_id, version1))
            })?;

        let v2 = self
            .repository
            .get_field_version(field_id, version2)?
            .ok_or_else(|| {
                VersioningError::VersionNotFound(format!("{}:{}", field_id, version2))
            })?;

        let time_diff = v2.created_at.signed_duration_since(v1.created_at);
        let value_changed = self.values_differ(&v1.field_value, &v2.field_value)?;
        let hash_changed = v1.metadata.content_hash != v2.metadata.content_hash;
        let size_changed = v1.metadata.size_bytes != v2.metadata.size_bytes;

        Ok(FieldVersionComparison {
            field_id,
            node_id: v1.node_id,
            version1: v1,
            version2: v2,
            value_changed,
            hash_changed,
            size_changed,
            time_diff,
        })
    }

    /// Restore a specific field version
    pub fn restore_field_version(
        &self,
        field_id: Uuid,
        version_number: u32,
        restored_by: Option<String>,
    ) -> Result<FieldVersion<T>, VersioningError> {
        let version_to_restore = self
            .repository
            .get_field_version(field_id, version_number)?
            .ok_or_else(|| {
                VersioningError::VersionNotFound(format!("{}:{}", field_id, version_number))
            })?;

        let field = Field {
            fid: field_id,
            nid: version_to_restore.node_id,
            value: version_to_restore.field_value,
        };

        self.create_field_version(
            &field,
            ChangeType::Restored,
            restored_by,
            Some(format!("Restored from version {}", version_number)),
        )
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

    fn validate_field_value(&self, _value: &T) -> ValidationStatus {
        // Basic validation - in practice, this would be more sophisticated
        ValidationStatus::Valid
    }

    fn values_differ(&self, value1: &T, value2: &T) -> Result<bool, VersioningError> {
        let hash1 = self.generate_content_hash(value1)?;
        let hash2 = self.generate_content_hash(value2)?;
        Ok(hash1 != hash2)
    }
}

#[derive(Debug, Clone)]
pub struct FieldVersionComparison<T> {
    pub field_id: Uuid,
    pub node_id: Uuid,
    pub version1: FieldVersion<T>,
    pub version2: FieldVersion<T>,
    pub value_changed: bool,
    pub hash_changed: bool,
    pub size_changed: bool,
    pub time_diff: chrono::Duration,
}

/// Type alias for field version storage
type FieldVersionMap<T> = Arc<RwLock<HashMap<(Uuid, u32), FieldVersion<T>>>>;

/// In-memory implementation for testing and development
pub struct InMemoryFieldVersionRepository<T> {
    versions: FieldVersionMap<T>,
    current_versions: Arc<RwLock<HashMap<Uuid, u32>>>,
}

impl<T> Default for InMemoryFieldVersionRepository<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> InMemoryFieldVersionRepository<T> {
    pub fn new() -> Self {
        Self {
            versions: Arc::new(RwLock::new(HashMap::new())),
            current_versions: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl<T> FieldVersionRepository<T> for InMemoryFieldVersionRepository<T>
where
    T: Clone,
{
    fn save_field_version(&self, version: &FieldVersion<T>) -> Result<(), VersioningError> {
        let mut versions = self
            .versions
            .write()
            .map_err(|e| VersioningError::StorageError(e.to_string()))?;
        let mut current_versions = self
            .current_versions
            .write()
            .map_err(|e| VersioningError::StorageError(e.to_string()))?;

        versions.insert((version.field_id, version.version_number), version.clone());

        if version.metadata.is_current {
            current_versions.insert(version.field_id, version.version_number);
        }

        Ok(())
    }

    fn get_field_version(
        &self,
        field_id: Uuid,
        version_number: u32,
    ) -> Result<Option<FieldVersion<T>>, VersioningError> {
        let versions = self
            .versions
            .read()
            .map_err(|e| VersioningError::StorageError(e.to_string()))?;
        Ok(versions.get(&(field_id, version_number)).cloned())
    }

    fn get_current_field_version(
        &self,
        field_id: Uuid,
    ) -> Result<Option<FieldVersion<T>>, VersioningError> {
        let current_versions = self
            .current_versions
            .read()
            .map_err(|e| VersioningError::StorageError(e.to_string()))?;
        let versions = self
            .versions
            .read()
            .map_err(|e| VersioningError::StorageError(e.to_string()))?;

        if let Some(&current_version_number) = current_versions.get(&field_id) {
            Ok(versions.get(&(field_id, current_version_number)).cloned())
        } else {
            Ok(None)
        }
    }

    fn get_field_version_history(
        &self,
        field_id: Uuid,
    ) -> Result<FieldVersionHistory<T>, VersioningError> {
        let versions = self
            .versions
            .read()
            .map_err(|e| VersioningError::StorageError(e.to_string()))?;
        let current_versions = self
            .current_versions
            .read()
            .map_err(|e| VersioningError::StorageError(e.to_string()))?;

        let mut field_versions: Vec<FieldVersion<T>> = versions
            .iter()
            .filter(|((id, _), _)| *id == field_id)
            .map(|(_, version)| version.clone())
            .collect();

        field_versions.sort_by_key(|a| a.version_number);

        let current_version = current_versions.get(&field_id).copied().unwrap_or(0);
        let total_versions = field_versions.len() as u32;
        // A field with no stored version has no history and no parent id to
        // report. Refuse instead of inventing a `node_id` (FIXES-02, D-03).
        let node_id = field_versions
            .first()
            .map(|v| v.node_id)
            .ok_or(VersioningError::EntityNotFound(field_id))?;

        Ok(FieldVersionHistory {
            field_id,
            node_id,
            current_version,
            versions: field_versions,
            total_versions,
        })
    }

    fn get_node_field_versions(
        &self,
        node_id: Uuid,
    ) -> Result<Vec<FieldVersion<T>>, VersioningError> {
        let versions = self
            .versions
            .read()
            .map_err(|e| VersioningError::StorageError(e.to_string()))?;

        let node_field_versions: Vec<FieldVersion<T>> = versions
            .values()
            .filter(|version| version.node_id == node_id)
            .cloned()
            .collect();

        Ok(node_field_versions)
    }

    fn delete_field_version(
        &self,
        field_id: Uuid,
        version_number: u32,
    ) -> Result<(), VersioningError> {
        let mut versions = self
            .versions
            .write()
            .map_err(|e| VersioningError::StorageError(e.to_string()))?;

        versions.remove(&(field_id, version_number));
        Ok(())
    }

    fn purge_old_field_versions(
        &self,
        field_id: Uuid,
        keep_count: u32,
    ) -> Result<u32, VersioningError> {
        let history = match self.get_field_version_history(field_id) {
            Ok(history) => history,
            // Nothing stored means nothing to purge, which is an honest `Ok(0)`.
            Err(VersioningError::EntityNotFound(_)) => return Ok(0),
            Err(other) => return Err(other),
        };

        if history.total_versions <= keep_count {
            return Ok(0);
        }

        let versions_to_delete = history.total_versions - keep_count;
        let mut deleted_count = 0;

        for version in history.versions.iter().take(versions_to_delete as usize) {
            self.delete_field_version(field_id, version.version_number)?;
            deleted_count += 1;
        }

        Ok(deleted_count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_field_version_creation() {
        let repository = Arc::new(InMemoryFieldVersionRepository::new());
        let service = FieldVersionService::new(repository, None);

        let field = Field::new("test_value".to_string());

        let version = service
            .create_field_version(
                &field,
                ChangeType::Created,
                Some("test_user".to_string()),
                Some("Initial field creation".to_string()),
            )
            .unwrap();

        assert_eq!(version.version_number, 1);
        assert_eq!(version.field_id, field.fid);
        assert!(version.metadata.is_current);
    }

    #[test]
    fn test_field_version_history() {
        let repository = Arc::new(InMemoryFieldVersionRepository::new());
        let service = FieldVersionService::new(repository, None);

        let mut field = Field::new("original".to_string());

        // Create initial version
        service
            .create_field_version(&field, ChangeType::Created, None, None)
            .unwrap();

        // Update field value
        field.value = "updated".to_string();
        service
            .create_field_version(&field, ChangeType::Updated, None, None)
            .unwrap();

        let history = service.get_field_version_history(field.fid).unwrap();
        assert_eq!(history.total_versions, 2);
        assert_eq!(history.current_version, 2);
        assert_eq!(history.node_id, field.nid);
    }

    #[test]
    fn field_version_history_of_an_unversioned_field_is_entity_not_found() {
        let repository = Arc::new(InMemoryFieldVersionRepository::<String>::new());
        let service = FieldVersionService::new(repository.clone(), None);
        let field_id = Uuid::new_v4();

        match service.get_field_version_history(field_id) {
            Err(VersioningError::EntityNotFound(id)) => assert_eq!(id, field_id),
            Err(other) => panic!("expected EntityNotFound, got {other:?}"),
            Ok(history) => panic!("expected a refusal, got a history for {}", history.node_id),
        }
        match repository.get_field_version_history(field_id) {
            Err(VersioningError::EntityNotFound(id)) => assert_eq!(id, field_id),
            Err(other) => panic!("expected EntityNotFound, got {other:?}"),
            Ok(history) => panic!("expected a refusal, got a history for {}", history.node_id),
        }
    }

    #[test]
    fn purging_an_unversioned_field_purges_nothing() {
        let repository = InMemoryFieldVersionRepository::<String>::new();

        let purged = repository
            .purge_old_field_versions(Uuid::new_v4(), 1)
            .unwrap();

        assert_eq!(purged, 0);
    }
}
