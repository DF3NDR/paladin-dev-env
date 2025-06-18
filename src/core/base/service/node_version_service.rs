/*
Node Version Service

The Node Version Service manages versioning for Node entities. It automatically creates
new versions when nodes are created or modified (if versioning is enabled), stores
historical versions, and provides retrieval methods for version history.

This service implements a revision system similar to Drupal's node revisions, where
each change creates a new version while preserving the complete history.
*/

use crate::core::base::entity::node::Node;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum VersioningError {
    #[error("Version not found: {0}")]
    VersionNotFound(String),
    #[error("Node not found: {0}")]
    NodeNotFound(Uuid),
    #[error("Invalid version number: {0}")]
    InvalidVersionNumber(u32),
    #[error("Versioning disabled for node: {0}")]
    VersioningDisabled(Uuid),
    #[error("Storage error: {0}")]
    StorageError(String),
    #[error("Serialization error: {0}")]
    SerializationError(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NodeVersion<T> {
    pub version_id: Uuid,
    pub node_id: Uuid,
    pub version_number: u32,
    pub node_data: T,
    pub metadata: NodeVersionMetadata,
    pub created_at: DateTime<Utc>,
    pub created_by: Option<String>,
    pub change_summary: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NodeVersionMetadata {
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
    pub node_id: Uuid,
    pub current_version: u32,
    pub versions: Vec<NodeVersion<T>>,
    pub total_versions: u32,
}

/// Node Version Repository trait
/// Defines the contract for persisting node versions
pub trait NodeVersionRepository<T> {
    fn save_version(&self, version: &NodeVersion<T>) -> Result<(), VersioningError>;
    fn get_version(&self, node_id: Uuid, version_number: u32) -> Result<Option<NodeVersion<T>>, VersioningError>;
    fn get_current_version(&self, node_id: Uuid) -> Result<Option<NodeVersion<T>>, VersioningError>;
    fn get_version_history(&self, node_id: Uuid) -> Result<VersionHistory<T>, VersioningError>;
    fn delete_version(&self, node_id: Uuid, version_number: u32) -> Result<(), VersioningError>;
    fn purge_old_versions(&self, node_id: Uuid, keep_count: u32) -> Result<u32, VersioningError>;
}

/// Node Version Service
/// Core service for managing node versioning
pub struct NodeVersionService<T> {
    repository: Arc<dyn NodeVersionRepository<T> + Send + Sync>,
    config: VersioningConfig,
}

#[derive(Debug, Clone)]
pub struct VersioningConfig {
    pub max_versions_per_node: Option<u32>,
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
            max_versions_per_node: Some(50),
            auto_purge_enabled: true,
            compress_old_versions: false,
            hash_algorithm: HashAlgorithm::Sha256,
        }
    }
}

impl<T> NodeVersionService<T> 
where 
    T: Clone + serde::Serialize + for<'de> serde::Deserialize<'de>,
{
    pub fn new(
        repository: Arc<dyn NodeVersionRepository<T> + Send + Sync>,
        config: Option<VersioningConfig>,
    ) -> Self {
        Self {
            repository,
            config: config.unwrap_or_default(),
        }
    }

    /// Create a new version when a node is created or modified
    pub fn create_version(
        &self,
        node: &Node<T>,
        change_type: ChangeType,
        created_by: Option<String>,
        change_summary: Option<String>,
    ) -> Result<NodeVersion<T>, VersioningError> {
        if !node.version {
            return Err(VersioningError::VersioningDisabled(node.uuid));
        }

        // Get current version number
        let current_version_number = match self.repository.get_current_version(node.uuid)? {
            Some(current) => current.version_number + 1,
            None => 1,
        };

        // Generate content hash
        let content_hash = self.generate_content_hash(&node.node)?;

        // Calculate serialized size
        let size_bytes = self.calculate_size(&node.node)?;

        let metadata = NodeVersionMetadata {
            is_published: matches!(change_type, ChangeType::Created | ChangeType::Published),
            is_current: true,
            tags: Vec::new(),
            change_type,
            content_hash,
            size_bytes: Some(size_bytes),
        };

        let version = NodeVersion {
            version_id: Uuid::new_v4(),
            node_id: node.uuid,
            version_number: current_version_number,
            node_data: node.node.clone(),
            metadata,
            created_at: Utc::now(),
            created_by,
            change_summary,
        };

        // Mark previous version as not current
        if let Some(mut current) = self.repository.get_current_version(node.uuid)? {
            current.metadata.is_current = false;
            self.repository.save_version(&current)?;
        }

        // Save new version
        self.repository.save_version(&version)?;

        // Auto-purge if enabled
        if self.config.auto_purge_enabled {
            if let Some(max_versions) = self.config.max_versions_per_node {
                if current_version_number > max_versions {
                    let _ = self.repository.purge_old_versions(node.uuid, max_versions);
                }
            }
        }

        Ok(version)
    }

    /// Get a specific version of a node
    pub fn get_version(&self, node_id: Uuid, version_number: u32) -> Result<Option<NodeVersion<T>>, VersioningError> {
        self.repository.get_version(node_id, version_number)
    }

    /// Get the current (latest) version of a node
    pub fn get_current_version(&self, node_id: Uuid) -> Result<Option<NodeVersion<T>>, VersioningError> {
        self.repository.get_current_version(node_id)
    }

    /// Get complete version history for a node
    pub fn get_version_history(&self, node_id: Uuid) -> Result<VersionHistory<T>, VersioningError> {
        self.repository.get_version_history(node_id)
    }

    /// Restore a specific version as the current version
    pub fn restore_version(
        &self,
        node_id: Uuid,
        version_number: u32,
        restored_by: Option<String>,
    ) -> Result<NodeVersion<T>, VersioningError> {
        let version_to_restore = self.repository.get_version(node_id, version_number)?
            .ok_or_else(|| VersioningError::VersionNotFound(format!("{}:{}", node_id, version_number)))?;

        // Create a new version based on the restored data
        let node = Node {
            uuid: node_id,
            created: Utc::now(), // This would normally come from the original node
            modified: Utc::now(),
            node: version_to_restore.node_data,
            name: None, // This would normally come from the original node
            version: true,
        };

        self.create_version(
            &node,
            ChangeType::Restored,
            restored_by,
            Some(format!("Restored from version {}", version_number)),
        )
    }

    /// Compare two versions and return differences
    pub fn compare_versions(
        &self,
        node_id: Uuid,
        version1: u32,
        version2: u32,
    ) -> Result<VersionComparison<T>, VersioningError> {
        let v1 = self.repository.get_version(node_id, version1)?
            .ok_or_else(|| VersioningError::VersionNotFound(format!("{}:{}", node_id, version1)))?;
        
        let v2 = self.repository.get_version(node_id, version2)?
            .ok_or_else(|| VersioningError::VersionNotFound(format!("{}:{}", node_id, version2)))?;

        let hash_changed = v1.metadata.content_hash != v2.metadata.content_hash;
        let size_changed = v1.metadata.size_bytes != v2.metadata.size_bytes;
        let time_diff = v2.created_at.signed_duration_since(v1.created_at);

        Ok(VersionComparison {
            node_id,
            version1: v1,
            version2: v2,
            hash_changed,
            size_changed,
            time_diff,
        })
    }

    /// Purge old versions manually
    pub fn purge_old_versions(&self, node_id: Uuid, keep_count: u32) -> Result<u32, VersioningError> {
        self.repository.purge_old_versions(node_id, keep_count)
    }

    fn generate_content_hash(&self, content: &T) -> Result<String, VersioningError> {
        let serialized = serde_json::to_vec(content)
            .map_err(|e| VersioningError::SerializationError(e.to_string()))?;

        match self.config.hash_algorithm {
            HashAlgorithm::Sha256 => {
                use sha2::{Sha256, Digest};
                let hash = Sha256::digest(&serialized);
                Ok(format!("{:x}", hash))
            },
            HashAlgorithm::Blake3 => {
                let hash = blake3::hash(&serialized);
                Ok(hash.to_hex().to_string())
            },
            HashAlgorithm::Xxhash => {
                use std::collections::hash_map::DefaultHasher;
                use std::hash::{Hash, Hasher};
                let mut hasher = DefaultHasher::new();
                serialized.hash(&mut hasher);
                Ok(format!("{:x}", hasher.finish()))
            },
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
    pub node_id: Uuid,
    pub version1: NodeVersion<T>,
    pub version2: NodeVersion<T>,
    pub hash_changed: bool,
    pub size_changed: bool,
    pub time_diff: chrono::Duration,
}

/// In-memory implementation for testing and development
pub struct InMemoryNodeVersionRepository<T> {
    versions: Arc<RwLock<HashMap<(Uuid, u32), NodeVersion<T>>>>,
    current_versions: Arc<RwLock<HashMap<Uuid, u32>>>,
}

impl<T> InMemoryNodeVersionRepository<T> {
    pub fn new() -> Self {
        Self {
            versions: Arc::new(RwLock::new(HashMap::new())),
            current_versions: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl<T> NodeVersionRepository<T> for InMemoryNodeVersionRepository<T>
where
    T: Clone,
{
    fn save_version(&self, version: &NodeVersion<T>) -> Result<(), VersioningError> {
        let mut versions = self.versions.write()
            .map_err(|e| VersioningError::StorageError(e.to_string()))?;
        let mut current_versions = self.current_versions.write()
            .map_err(|e| VersioningError::StorageError(e.to_string()))?;

        versions.insert((version.node_id, version.version_number), version.clone());
        
        if version.metadata.is_current {
            current_versions.insert(version.node_id, version.version_number);
        }

        Ok(())
    }

    fn get_version(&self, node_id: Uuid, version_number: u32) -> Result<Option<NodeVersion<T>>, VersioningError> {
        let versions = self.versions.read()
            .map_err(|e| VersioningError::StorageError(e.to_string()))?;
        Ok(versions.get(&(node_id, version_number)).cloned())
    }

    fn get_current_version(&self, node_id: Uuid) -> Result<Option<NodeVersion<T>>, VersioningError> {
        let current_versions = self.current_versions.read()
            .map_err(|e| VersioningError::StorageError(e.to_string()))?;
        let versions = self.versions.read()
            .map_err(|e| VersioningError::StorageError(e.to_string()))?;

        if let Some(&current_version_number) = current_versions.get(&node_id) {
            Ok(versions.get(&(node_id, current_version_number)).cloned())
        } else {
            Ok(None)
        }
    }

    fn get_version_history(&self, node_id: Uuid) -> Result<VersionHistory<T>, VersioningError> {
        let versions = self.versions.read()
            .map_err(|e| VersioningError::StorageError(e.to_string()))?;
        let current_versions = self.current_versions.read()
            .map_err(|e| VersioningError::StorageError(e.to_string()))?;

        let mut node_versions: Vec<NodeVersion<T>> = versions
            .iter()
            .filter(|((id, _), _)| *id == node_id)
            .map(|(_, version)| version.clone())
            .collect();

        node_versions.sort_by(|a, b| a.version_number.cmp(&b.version_number));

        let current_version = current_versions.get(&node_id).copied().unwrap_or(0);
        let total_versions = node_versions.len() as u32;

        Ok(VersionHistory {
            node_id,
            current_version,
            versions: node_versions,
            total_versions,
        })
    }

    fn delete_version(&self, node_id: Uuid, version_number: u32) -> Result<(), VersioningError> {
        let mut versions = self.versions.write()
            .map_err(|e| VersioningError::StorageError(e.to_string()))?;
        
        versions.remove(&(node_id, version_number));
        Ok(())
    }

    fn purge_old_versions(&self, node_id: Uuid, keep_count: u32) -> Result<u32, VersioningError> {
        let history = self.get_version_history(node_id)?;
        
        if history.total_versions <= keep_count {
            return Ok(0);
        }

        let versions_to_delete = history.total_versions - keep_count;
        let mut deleted_count = 0;

        // Delete oldest versions, keeping the most recent ones
        for version in history.versions.iter().take(versions_to_delete as usize) {
            self.delete_version(node_id, version.version_number)?;
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
    fn test_node_version_creation() {
        let repository = Arc::new(InMemoryNodeVersionRepository::new());
        let service = NodeVersionService::new(repository, None);

        let test_data = TestData {
            value: "test".to_string(),
            number: 42,
        };

        let node = Node::new(test_data, Some("test_node".to_string()));

        let version = service.create_version(
            &node,
            ChangeType::Created,
            Some("test_user".to_string()),
            Some("Initial creation".to_string()),
        ).unwrap();

        assert_eq!(version.version_number, 1);
        assert_eq!(version.node_id, node.uuid);
        assert!(version.metadata.is_current);
        assert_eq!(version.metadata.change_type, ChangeType::Created);
    }

    #[test]
    fn test_version_history() {
        let repository = Arc::new(InMemoryNodeVersionRepository::new());
        let service = NodeVersionService::new(repository, None);

        let test_data = TestData {
            value: "test".to_string(),
            number: 42,
        };

        let node = Node::new(test_data.clone(), Some("test_node".to_string()));

        // Create initial version
        service.create_version(&node, ChangeType::Created, None, None).unwrap();

        // Create updated version
        let updated_data = TestData {
            value: "updated".to_string(),
            number: 43,
        };
        let updated_node = Node {
            node: updated_data,
            ..node
        };
        
        service.create_version(&updated_node, ChangeType::Updated, None, None).unwrap();

        let history = service.get_version_history(node.uuid).unwrap();
        assert_eq!(history.total_versions, 2);
        assert_eq!(history.current_version, 2);
    }

    #[test]
    fn test_version_restore() {
        let repository = Arc::new(InMemoryNodeVersionRepository::new());
        let service = NodeVersionService::new(repository, None);

        let test_data = TestData {
            value: "original".to_string(),
            number: 1,
        };

        let node = Node::new(test_data.clone(), Some("test_node".to_string()));

        // Create initial version
        service.create_version(&node, ChangeType::Created, None, None).unwrap();

        // Create updated version
        let updated_data = TestData {
            value: "updated".to_string(),
            number: 2,
        };
        let updated_node = Node {
            node: updated_data,
            ..node
        };
        
        service.create_version(&updated_node, ChangeType::Updated, None, None).unwrap();

        // Restore original version
        let restored = service.restore_version(node.uuid, 1, Some("test_user".to_string())).unwrap();
        
        assert_eq!(restored.version_number, 3);
        assert_eq!(restored.metadata.change_type, ChangeType::Restored);
    }
}