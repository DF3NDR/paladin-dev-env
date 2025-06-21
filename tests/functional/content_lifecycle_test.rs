#[cfg(test)]
mod functional_tests {
    use in4me::core::platform::container::content::{ContentItem, ContentType, TextContent};
    use in4me::core::base::service::node_version_service::{
        InMemoryNodeVersionRepository, ChangeType, VersioningError, NodeVersionRepository
    };
    use in4me::core::platform::container::content_service::{ContentItemService, ContentItemServiceError};
    use std::sync::Arc;
    use uuid::Uuid;
    use chrono::Utc;

    #[test]
    fn test_functional_content_item_lifecycle() {
        // Setup
        let repository: Arc<dyn NodeVersionRepository<_> + Send + Sync> = Arc::new(InMemoryNodeVersionRepository::new());
        let service = ContentItemService::new(repository.clone());
        
        // Create initial content
        let text_content = TextContent::new(
            None, 
            Some("This is my first blog post about functional testing.".to_string())
        ).unwrap();
        
        let content_item = ContentItem::new_with_title(
            ContentType::Text(text_content),
            "My First Blog Post".to_string(),
        ).unwrap();
        
        let content_id = content_item.uuid();
        
        // Test 1: Create content item with initial version
        let (mut content_item, initial_version) = service.create_content_item(
            content_item,
            Some("author@example.com".to_string()),
            Some("Initial blog post creation".to_string()),
        ).unwrap();
        
        assert_eq!(initial_version.version_number, 1);
        assert_eq!(initial_version.metadata.change_type, ChangeType::Created);
        assert_eq!(initial_version.created_by, Some("author@example.com".to_string()));
        assert!(initial_version.metadata.is_current);
        
        // Test 2: Update content metadata
        content_item.set_description(Some("A blog post about functional testing in Rust".to_string()));
        content_item.set_tags(Some(vec!["rust".to_string(), "testing".to_string(), "functional".to_string()]));
        content_item.set_author(Some("John Doe".to_string()));
        
        let (mut content_item, update_version) = service.update_content_item(
            content_item,
            Some("editor@example.com".to_string()),
            Some("Added metadata and tags".to_string()),
        ).unwrap();
        
        assert_eq!(update_version.version_number, 2);
        assert_eq!(update_version.metadata.change_type, ChangeType::Updated);
        assert_eq!(update_version.created_by, Some("editor@example.com".to_string()));
        
        // Test 3: Update content text
        let updated_text_content = TextContent::new(
            None,
            Some("This is my first blog post about functional testing. I've updated it with more details about how functional tests work in Rust.".to_string())
        ).unwrap();
        
        content_item.update_content(ContentType::Text(updated_text_content)).unwrap();
        
        let (content_item, content_update_version) = service.update_content_item(
            content_item,
            Some("author@example.com".to_string()),
            Some("Expanded content with more details".to_string()),
        ).unwrap();
        
        assert_eq!(content_update_version.version_number, 3);
        
        // Test 4: Publish the content
        let publish_version = service.publish_content_item(
            &content_item,
            Some("publisher@example.com".to_string()),
        ).unwrap();
        
        assert_eq!(publish_version.version_number, 4);
        assert_eq!(publish_version.metadata.change_type, ChangeType::Published);
        assert!(publish_version.metadata.is_published);
        
        // Test 5: Get version history
        let history = service.get_version_history(content_id).unwrap();
        
        assert_eq!(history.total_versions, 4);
        assert_eq!(history.current_version, 4);
        assert_eq!(history.node_id, content_id);
        
        // Verify all versions are tracked
        let version_types: Vec<ChangeType> = history.versions.iter()
            .map(|v| v.metadata.change_type.clone())
            .collect();
        
        assert_eq!(version_types, vec![
            ChangeType::Created,
            ChangeType::Updated,
            ChangeType::Updated,
            ChangeType::Published,
        ]);
        
        // Test 6: Get specific version
        let version_2 = service.get_version(content_id, 2).unwrap().unwrap();
        assert_eq!(version_2.change_summary, Some("Added metadata and tags".to_string()));
        assert_eq!(version_2.created_by, Some("editor@example.com".to_string()));
        
        // Test 7: Compare versions
        let comparison = service.compare_versions(content_id, 1, 3).unwrap();
        assert!(comparison.hash_changed);
        assert_ne!(comparison.version1.node_data.content, comparison.version2.node_data.content);
        
        // Test 8: Restore to previous version
        let (restored_content, restore_version) = service.restore_version(
            content_id,
            2,
            Some("admin@example.com".to_string()),
        ).unwrap();
        
        assert_eq!(restore_version.version_number, 5);
        assert_eq!(restore_version.metadata.change_type, ChangeType::Restored);
        assert!(restore_version.change_summary.unwrap().contains("Restored from version 2"));
        
        // Test 9: Final version history check
        let final_history = service.get_version_history(content_id).unwrap();
        assert_eq!(final_history.total_versions, 5);
        assert_eq!(final_history.current_version, 5);
        
        // Test 10: Archive the content
        let archive_version = service.archive_content_item(
            &restored_content,
            Some("admin@example.com".to_string()),
        ).unwrap();
        
        assert_eq!(archive_version.version_number, 6);
        assert_eq!(archive_version.metadata.change_type, ChangeType::Archived);
        
        println!("✅ Functional test completed successfully!");
        println!("   - Created content item with versioning");
        println!("   - Updated metadata and content");
        println!("   - Published content");
        println!("   - Tracked complete version history");
        println!("   - Restored previous version");
        println!("   - Archived content");
        println!("   - Total versions created: 6");
    }

    #[test]
    fn test_versioning_disabled_content() {
        let repository = Arc::new(InMemoryNodeVersionRepository::new());
        let service = ContentItemService::new(repository.clone());
        
        let text_content = TextContent::new(
            None, 
            Some("This content will not be versioned.".to_string())
        ).unwrap();
        
        let mut content_item = ContentItem::new_with_title(
            ContentType::Text(text_content),
            "Unversioned Content".to_string(),
        ).unwrap();
        
        // Disable versioning
        content_item.disable_versioning();
        assert!(!content_item.versioning_enabled());
        
        // Try to create version - should fail
        let result = service.create_content_item(
            content_item,
            Some("author@example.com".to_string()),
            Some("Attempt to create version".to_string()),
        );
        
        assert!(result.is_err());
        match result.unwrap_err() {
            ContentItemServiceError::VersioningError(VersioningError::VersioningDisabled(_)) => {
                println!("✅ Correctly prevented versioning for disabled content");
            },
            other => panic!("Expected VersioningDisabled error, got: {:?}", other),
        }
    }

    #[test]
    fn test_content_node_integration() {
        // Test that ContentItem properly integrates with Node structure
        let text_content = TextContent::new(
            None,
            Some("Testing node integration".to_string())
        ).unwrap();
        
        let content_item = ContentItem::new_with_title(
            ContentType::Text(text_content),
            "Node Integration Test".to_string(),
        ).unwrap();
        
        // Test node properties
        assert_eq!(content_item.node.uuid, content_item.uuid());
        assert_eq!(content_item.node.created, content_item.created());
        assert_eq!(content_item.node.modified, content_item.modified());
        assert_eq!(content_item.node.name, content_item.title().cloned());
        assert_eq!(content_item.node.version, content_item.versioning_enabled());
        
        // Test that the node contains the content data
        match content_item.content() {
            ContentType::Text(text) => {
                assert_eq!(text.content, Some("Testing node integration".to_string()));
                assert_eq!(content_item.node.node.content, *content_item.content());
            },
            _ => panic!("Expected text content"),
        }
        
        println!("✅ Node integration test passed");
    }

    #[test]
    fn test_content_item_basic_functionality() {
        // Simple test to verify ContentItem works without versioning service
        let text_content = TextContent::new(
            None,
            Some("Basic functionality test".to_string())
        ).unwrap();
        
        let mut content_item = ContentItem::new_with_title(
            ContentType::Text(text_content),
            "Basic Test".to_string(),
        ).unwrap();
        
        // Test basic properties
        assert_eq!(content_item.title(), Some(&"Basic Test".to_string()));
        assert!(content_item.uuid() != Uuid::nil());
        assert!(content_item.created() <= Utc::now());
        assert!(content_item.versioning_enabled());
        
        // Test property updates
        content_item.set_description(Some("Updated description".to_string()));
        content_item.set_author(Some("Test Author".to_string()));
        content_item.set_tags(Some(vec!["tag1".to_string(), "tag2".to_string()]));
        
        assert_eq!(content_item.description(), Some(&"Updated description".to_string()));
        assert_eq!(content_item.author(), Some(&"Test Author".to_string()));
        assert_eq!(content_item.tags(), Some(&vec!["tag1".to_string(), "tag2".to_string()]));
        
        // Test versioning control
        content_item.disable_versioning();
        assert!(!content_item.versioning_enabled());
        
        content_item.enable_versioning();
        assert!(content_item.versioning_enabled());
        
        println!("✅ Basic functionality test passed");
    }

    #[test]
    fn test_content_type_variations() {
        use in4me::core::platform::container::content::{VideoContent, AudioContent, ImageContent};
        
        // Test Text Content
        let text_content = TextContent::new(None, Some("Text content".to_string())).unwrap();
        let text_item = ContentItem::new(ContentType::Text(text_content)).unwrap();
        assert!(matches!(*text_item.content(), ContentType::Text(_)));
        
        // Test Video Content
        let video_content = VideoContent::new(None, 3600).unwrap(); // 1 hour duration
        let video_item = ContentItem::new(ContentType::Video(video_content)).unwrap();
        assert!(matches!(*video_item.content(), ContentType::Video(_)));
        
        // Test Audio Content
        let audio_content = AudioContent::new(None, 180).unwrap(); // 3 minutes duration
        let audio_item = ContentItem::new(ContentType::Audio(audio_content)).unwrap();
        assert!(matches!(*audio_item.content(), ContentType::Audio(_)));
        
        // Test Image Content
        let image_content = ImageContent::new(None, (1920, 1080)).unwrap(); // Full HD resolution
        let image_item = ContentItem::new(ContentType::Image(image_content)).unwrap();
        assert!(matches!(*image_item.content(), ContentType::Image(_)));
        
        println!("✅ Content type variations test passed");
    }
}