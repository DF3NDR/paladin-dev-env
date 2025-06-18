use crate::core::platform::container::content::{ContentItem, ContentItemError, ContentData};
use crate::core::base::service::node_version_service::{
    NodeVersionService, NodeVersionRepository, ChangeType, VersioningError, NodeVersion, VersionHistory
};
use uuid::Uuid;
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum ContentItemServiceError {
    #[error("Content item error: {0}")]
    ContentItemError(#[from] ContentItemError),
    #[error("Versioning error: {0}")]
    VersioningError(#[from] VersioningError),
    #[error("Content item not found: {0}")]
    ContentItemNotFound(Uuid),
}

pub struct ContentItemService {
    version_service: NodeVersionService<ContentData>,
}

impl ContentItemService {
    pub fn new(
        version_repository: Arc<dyn NodeVersionRepository<ContentData> + Send + Sync>,
    ) -> Self {
        Self {
            version_service: NodeVersionService::new(version_repository, None),
        }
    }

    /// Create a new content item and its initial version
    pub fn create_content_item(
        &self,
        content_item: ContentItem,
        created_by: Option<String>,
        change_summary: Option<String>,
    ) -> Result<(ContentItem, NodeVersion<ContentData>), ContentItemServiceError> {
        // Create initial version
        let version = self.version_service.create_version(
            &content_item.node,
            ChangeType::Created,
            created_by,
            change_summary,
        )?;

        Ok((content_item, version))
    }

    /// Update a content item and create a new version
    pub fn update_content_item(
        &self,
        content_item: ContentItem,
        updated_by: Option<String>,
        change_summary: Option<String>,
    ) -> Result<(ContentItem, NodeVersion<ContentData>), ContentItemServiceError> {
        // Create new version
        let version = self.version_service.create_version(
            &content_item.node,
            ChangeType::Updated,
            updated_by,
            change_summary,
        )?;

        Ok((content_item, version))
    }

    /// Get the current version of a content item
    pub fn get_current_version(
        &self,
        content_id: Uuid,
    ) -> Result<Option<NodeVersion<ContentData>>, ContentItemServiceError> {
        Ok(self.version_service.get_current_version(content_id)?)
    }

    /// Get a specific version of a content item
    pub fn get_version(
        &self,
        content_id: Uuid,
        version_number: u32,
    ) -> Result<Option<NodeVersion<ContentData>>, ContentItemServiceError> {
        Ok(self.version_service.get_version(content_id, version_number)?)
    }

    /// Get version history for a content item
    pub fn get_version_history(
        &self,
        content_id: Uuid,
    ) -> Result<VersionHistory<ContentData>, ContentItemServiceError> {
        Ok(self.version_service.get_version_history(content_id)?)
    }

    /// Restore a specific version of a content item
    pub fn restore_version(
        &self,
        content_id: Uuid,
        version_number: u32,
        restored_by: Option<String>,
    ) -> Result<(ContentItem, NodeVersion<ContentData>), ContentItemServiceError> {
        let restored_version = self.version_service.restore_version(
            content_id,
            version_number,
            restored_by,
        )?;

        // Reconstruct ContentItem from the restored version
        let content_item = ContentItem {
            node: crate::core::base::entity::node::Node {
                uuid: content_id,
                created: restored_version.created_at, // This should be the original creation time
                modified: restored_version.created_at,
                node: restored_version.node_data.clone(),
                name: None, // This would need to be tracked in the version
                version: true,
            },
        };

        Ok((content_item, restored_version))
    }

    /// Publish a content item version
    pub fn publish_content_item(
        &self,
        content_item: &ContentItem,
        published_by: Option<String>,
    ) -> Result<NodeVersion<ContentData>, ContentItemServiceError> {
        let version = self.version_service.create_version(
            &content_item.node,
            ChangeType::Published,
            published_by,
            Some("Content published".to_string()),
        )?;

        Ok(version)
    }

    /// Unpublish a content item version
    pub fn unpublish_content_item(
        &self,
        content_item: &ContentItem,
        unpublished_by: Option<String>,
    ) -> Result<NodeVersion<ContentData>, ContentItemServiceError> {
        let version = self.version_service.create_version(
            &content_item.node,
            ChangeType::Unpublished,
            unpublished_by,
            Some("Content unpublished".to_string()),
        )?;

        Ok(version)
    }

    /// Archive a content item version
    pub fn archive_content_item(
        &self,
        content_item: &ContentItem,
        archived_by: Option<String>,
    ) -> Result<NodeVersion<ContentData>, ContentItemServiceError> {
        let version = self.version_service.create_version(
            &content_item.node,
            ChangeType::Archived,
            archived_by,
            Some("Content archived".to_string()),
        )?;

        Ok(version)
    }

    /// Compare two versions of a content item
    pub fn compare_versions(
        &self,
        content_id: Uuid,
        version1: u32,
        version2: u32,
    ) -> Result<crate::core::base::service::node_version_service::VersionComparison<ContentData>, ContentItemServiceError> {
        Ok(self.version_service.compare_versions(content_id, version1, version2)?)
    }

    /// Purge old versions of a content item
    pub fn purge_old_versions(
        &self,
        content_id: Uuid,
        keep_count: u32,
    ) -> Result<u32, ContentItemServiceError> {
        Ok(self.version_service.purge_old_versions(content_id, keep_count)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::platform::container::content::{ContentType, TextContent};
    use crate::core::base::service::node_version_service::InMemoryNodeVersionRepository;
    use std::sync::Arc;

    fn create_test_service() -> ContentItemService {
        let repository = Arc::new(InMemoryNodeVersionRepository::new());
        ContentItemService::new(repository)
    }

    fn create_test_content_item() -> ContentItem {
        let text_content = TextContent::new(None, Some("test content".to_string())).unwrap();
        ContentItem::new_with_title(
            ContentType::Text(text_content),
            "Test Content".to_string(),
        ).unwrap()
    }

    #[test]
    fn test_create_content_item() {
        let service = create_test_service();
        let content_item = create_test_content_item();
        
        let result = service.create_content_item(
            content_item.clone(),
            Some("test_user".to_string()),
            Some("Initial creation".to_string()),
        );
        
        assert!(result.is_ok());
        let (returned_item, version) = result.unwrap();
        
        assert_eq!(returned_item.uuid(), content_item.uuid());
        assert_eq!(version.version_number, 1);
        assert_eq!(version.node_id, content_item.uuid());
        assert_eq!(version.metadata.change_type, ChangeType::Created);
    }

    #[test]
    fn test_update_content_item() {
        let service = create_test_service();
        let content_item = create_test_content_item();
        
        // Create initial version
        let (mut content_item, _) = service.create_content_item(
            content_item,
            Some("test_user".to_string()),
            Some("Initial creation".to_string()),
        ).unwrap();
        
        // Update the content
        let new_text_content = TextContent::new(None, Some("updated content".to_string())).unwrap();
        content_item.update_content(ContentType::Text(new_text_content)).unwrap();
        
        // Create updated version
        let result = service.update_content_item(
            content_item.clone(),
            Some("test_user".to_string()),
            Some("Content updated".to_string()),
        );
        
        assert!(result.is_ok());
        let (_, version) = result.unwrap();
        
        assert_eq!(version.version_number, 2);
        assert_eq!(version.metadata.change_type, ChangeType::Updated);
    }

    #[test]
    fn test_get_version_history() {
        let service = create_test_service();
        let content_item = create_test_content_item();
        
        // Create initial version
        let (mut content_item, _) = service.create_content_item(
            content_item,
            Some("test_user".to_string()),
            None,
        ).unwrap();
        
        // Update the content
        let new_text_content = TextContent::new(None, Some("updated content".to_string())).unwrap();
        content_item.update_content(ContentType::Text(new_text_content)).unwrap();
        
        // Create updated version
        service.update_content_item(
            content_item.clone(),
            Some("test_user".to_string()),
            None,
        ).unwrap();
        
        // Get version history
        let history = service.get_version_history(content_item.uuid()).unwrap();
        
        assert_eq!(history.total_versions, 2);
        assert_eq!(history.current_version, 2);
        assert_eq!(history.versions.len(), 2);
    }

    #[test]
    fn test_restore_version() {
        let service = create_test_service();
        let content_item = create_test_content_item();
        
        // Create initial version
        let (mut content_item, _) = service.create_content_item(
            content_item,
            Some("test_user".to_string()),
            None,
        ).unwrap();
        
        // Update the content
        let new_text_content = TextContent::new(None, Some("updated content".to_string())).unwrap();
        content_item.update_content(ContentType::Text(new_text_content)).unwrap();
        
        // Create updated version
        service.update_content_item(
            content_item.clone(),
            Some("test_user".to_string()),
            None,
        ).unwrap();
        
        // Restore to version 1
        let result = service.restore_version(
            content_item.uuid(),
            1,
            Some("test_user".to_string()),
        );
        
        assert!(result.is_ok());
        let (_, restored_version) = result.unwrap();
        
        assert_eq!(restored_version.version_number, 3);
        assert_eq!(restored_version.metadata.change_type, ChangeType::Restored);
    }

    #[test]
    fn test_publish_unpublish_archive() {
        let service = create_test_service();
        let content_item = create_test_content_item();
        
        // Create initial version
        let (content_item, _) = service.create_content_item(
            content_item,
            Some("test_user".to_string()),
            None,
        ).unwrap();
        
        // Publish
        let publish_result = service.publish_content_item(
            &content_item,
            Some("test_user".to_string()),
        );
        assert!(publish_result.is_ok());
        let publish_version = publish_result.unwrap();
        assert_eq!(publish_version.metadata.change_type, ChangeType::Published);
        
        // Unpublish
        let unpublish_result = service.unpublish_content_item(
            &content_item,
            Some("test_user".to_string()),
        );
        assert!(unpublish_result.is_ok());
        let unpublish_version = unpublish_result.unwrap();
        assert_eq!(unpublish_version.metadata.change_type, ChangeType::Unpublished);
        
        // Archive
        let archive_result = service.archive_content_item(
            &content_item,
            Some("test_user".to_string()),
        );
        assert!(archive_result.is_ok());
        let archive_version = archive_result.unwrap();
        assert_eq!(archive_version.metadata.change_type, ChangeType::Archived);
    }

    #[test]
    fn test_versioning_disabled() {
        let service = create_test_service();
        let mut content_item = create_test_content_item();
        
        // Disable versioning
        content_item.disable_versioning();
        
        // Try to create version - should fail
        let result = service.create_content_item(
            content_item,
            Some("test_user".to_string()),
            None,
        );
        
        assert!(result.is_err());
        if let Err(ContentItemServiceError::VersioningError(VersioningError::VersioningDisabled(_))) = result {
            // Expected error
        } else {
            panic!("Expected VersioningDisabled error");
        }
    }
}