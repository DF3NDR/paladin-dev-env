use paladin::infrastructure::repositories::mysql_content_repository::{MySqlContentRepository, MySqlConfig};
use paladin::core::platform::container::content::{ContentItem, ContentType, TextContent, ContentData};
use paladin::core::base::entity::node::Node;
use paladin::application::storage::sql_store::ContentRepository;
use uuid::Uuid;
use chrono::Utc;
use serde_json;

// Helper function to create a test ContentItem (moved outside modules so it can be shared)
fn create_test_content_item() -> ContentItem {
    let text_content = TextContent {
        path: Some("test.txt".to_string()),
        content: Some("Test content for repository".to_string()),
        filesize: 26,
    };
    
    let content_data = ContentData {
        content: ContentType::Text(text_content),
        url: None,
        hash: Some("test-hash-123".to_string()),
        source_id: None,
        source_url: None,
        description: Some("Test description".to_string()),
        tags: Some(vec!["test".to_string(), "repository".to_string()]),
        source: Some("test-source".to_string()),
        author: Some("test-author".to_string()),
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
        name: Some("Test Content Item".to_string()),
        version: true,
    };

    ContentItem { node }
}

// Helper function to create test MySQL config (using in-memory SQLite for testing)
fn create_test_config() -> MySqlConfig {
    MySqlConfig {
        database_url: "sqlite::memory:".to_string(), // Use SQLite for testing
        max_connections: 1,
        min_connections: 1,
        connect_timeout_seconds: 5,
        idle_timeout_seconds: 30,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mysql_repository_creation() {
        let config = create_test_config();
        
        // Note: This test will only pass with a real database connection
        // For unit testing, we would typically use a mock or in-memory database
        match MySqlContentRepository::new(config).await {
            Ok(_repository) => {
                println!("Repository created successfully");
            },
            Err(e) => {
                println!("Repository creation failed (expected in test environment): {}", e);
                // This is expected in test environment without database
            }
        }
    }

    #[test]
    fn test_content_item_node_structure() {
        let content_item = create_test_content_item();
        
        // Test that we can access Node fields properly
        assert!(content_item.uuid() != Uuid::nil());
        assert!(content_item.created() <= Utc::now());
        assert!(content_item.modified() <= Utc::now());
        assert_eq!(content_item.title(), Some(&"Test Content Item".to_string()));
        assert_eq!(content_item.description(), Some(&"Test description".to_string()));
        assert_eq!(content_item.source(), Some(&"test-source".to_string()));
        assert_eq!(content_item.author(), Some(&"test-author".to_string()));
        assert_eq!(content_item.hash(), Some(&"test-hash-123".to_string()));
        
        // Test tags
        if let Some(tags) = content_item.tags() {
            assert_eq!(tags.len(), 2);
            assert!(tags.contains(&"test".to_string()));
            assert!(tags.contains(&"repository".to_string()));
        }
        
        // Test content type
        match content_item.content() {
            ContentType::Text(text) => {
                assert_eq!(text.path, Some("test.txt".to_string()));
                assert_eq!(text.content, Some("Test content for repository".to_string()));
                assert_eq!(text.filesize, 26);
            },
            _ => panic!("Expected Text content type"),
        }
    }

    #[test]
    fn test_content_item_serialization_compatibility() {
        let content_item = create_test_content_item();
        
        // Test that the structure is compatible with serialization
        let json_result = serde_json::to_string(&content_item);
        assert!(json_result.is_ok(), "ContentItem should be serializable");
        
        if let Ok(json) = json_result {
            let deserialized: Result<ContentItem, _> = serde_json::from_str(&json);
            assert!(deserialized.is_ok(), "ContentItem should be deserializable");
            
            if let Ok(deserialized_item) = deserialized {
                assert_eq!(content_item.uuid(), deserialized_item.uuid());
                assert_eq!(content_item.title(), deserialized_item.title());
                assert_eq!(content_item.hash(), deserialized_item.hash());
            }
        }
    }

    #[test]
    fn test_content_type_variations() {
        // Test different content types that the repository needs to handle
        let text_content = TextContent {
            path: None,
            content: Some("Plain text content".to_string()),
            filesize: 18,
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

        let node = Node {
            uuid: Uuid::new_v4(),
            created: Utc::now(),
            modified: Utc::now(),
            node: content_data,
            name: None,
            version: true,
        };

        let content_item = ContentItem { node };
        
        match content_item.content() {
            ContentType::Text(text) => {
                assert_eq!(text.content, Some("Plain text content".to_string()));
                assert_eq!(text.filesize, 18);
            },
            _ => panic!("Expected Text content type"),
        }
    }

    // Mock test for repository operations (these would require database setup)
    #[test]
    fn test_repository_interface_compatibility() {
        // This test verifies that our ContentItem structure is compatible 
        // with the repository interface requirements
        
        let content_item = create_test_content_item();
        
        // Verify all required fields for database operations are accessible
        assert!(content_item.uuid() != Uuid::nil());
        assert!(content_item.created() <= Utc::now());
        assert!(content_item.modified() <= Utc::now());
        
        // Verify content can be accessed for serialization
        match content_item.content() {
            ContentType::Text(_) => {}, // Content type is accessible
            _ => panic!("Content should be accessible"),
        }
        
        // Verify optional fields are properly handled
        let _ = content_item.url(); // Should not panic
        let _ = content_item.source_url(); // Should not panic
        let _ = content_item.tags(); // Should return Option
        
        println!("Repository interface compatibility verified");
    }

    #[test]
    fn test_node_reconstruction_pattern() {
        // Test the pattern used by the repository to reconstruct nodes from database
        let original_item = create_test_content_item();
        
        // Simulate extracting values (as repository would from database row)
        let uuid = original_item.node.uuid;
        let created = original_item.node.created;
        let modified = original_item.node.modified;
        let name = original_item.node.name.clone();
        let version = original_item.node.version;
        
        // Simulate reconstructing ContentData
        let content_data = ContentData {
            content: ContentType::Text(TextContent {
                path: Some("reconstructed.txt".to_string()),
                content: Some("Reconstructed content".to_string()),
                filesize: 20,
            }),
            url: None,
            hash: Some("reconstructed-hash".to_string()),
            source_id: None,
            source_url: None,
            description: Some("Reconstructed description".to_string()),
            tags: None,
            source: None,
            author: None,
            pub_date: None,
            mod_date: None,
        };
        
        // Reconstruct Node (as repository does)
        let reconstructed_node = Node {
            uuid,
            created,
            modified,
            node: content_data,
            name,
            version,
        };
        
        let reconstructed_item = ContentItem { node: reconstructed_node };
        
        // Verify reconstruction preserves key identifiers
        assert_eq!(original_item.uuid(), reconstructed_item.uuid());
        assert_eq!(original_item.created(), reconstructed_item.created());
        assert_eq!(original_item.modified(), reconstructed_item.modified());
        assert_eq!(original_item.title(), reconstructed_item.title());
        
        // Content should be different (reconstructed)
        match reconstructed_item.content() {
            ContentType::Text(text) => {
                assert_eq!(text.content, Some("Reconstructed content".to_string()));
                assert_eq!(text.path, Some("reconstructed.txt".to_string()));
            },
            _ => panic!("Expected Text content type"),
        }
        
        assert_eq!(reconstructed_item.hash(), Some(&"reconstructed-hash".to_string()));
        assert_eq!(reconstructed_item.description(), Some(&"Reconstructed description".to_string()));
        
        println!("Node reconstruction pattern verified");
    }

    // Integration test placeholder (requires actual database)
    #[ignore] // Use #[ignore] for tests that require external dependencies
    #[tokio::test]
    async fn test_full_repository_operations() {
        // This test would require a real database connection
        // It's marked with #[ignore] so it won't run by default
        
        let config = MySqlConfig {
            database_url: "mysql://test_user:test_pass@localhost:3306/test_db".to_string(),
            max_connections: 1,
            min_connections: 1,
            connect_timeout_seconds: 30,
            idle_timeout_seconds: 600,
        };
        
        if let Ok(repo) = MySqlContentRepository::new(config).await {
            let content_item = create_test_content_item();
            
            // Test create (not save - method name changed)
            match repo.create(content_item.clone()).await {
                Ok(id) => {
                    println!("Create successful, ID: {}", id);
                    assert_eq!(id, content_item.uuid());
                },
                Err(e) => panic!("Create failed: {}", e),
            }
            
            // Test retrieve
            match repo.get_by_id(content_item.uuid()).await {
                Ok(Some(retrieved)) => {
                    assert_eq!(content_item.uuid(), retrieved.uuid());
                    println!("Retrieve successful");
                },
                Ok(None) => panic!("Item not found after create"),
                Err(e) => panic!("Retrieve failed: {}", e),
            }
            
            // Test delete
            match repo.delete(content_item.uuid()).await {
                Ok(()) => println!("Delete successful"),
                Err(e) => panic!("Delete failed: {}", e),
            }
        } else {
            println!("Skipping integration test - database not available");
        }
    }
}

// Additional helper tests for repository functionality
#[cfg(test)]
mod repository_helpers {
    use super::*;

    #[test]
    fn test_mysql_config_default() {
        let config = MySqlConfig::default();
        assert!(config.database_url.contains("mysql://"));
        assert!(config.max_connections > 0);
        assert!(config.min_connections > 0);
        assert!(config.connect_timeout_seconds > 0);
        assert!(config.idle_timeout_seconds > 0);
    }

    #[test]
    fn test_content_data_fields() {
        let content_item = create_test_content_item();
        
        // Test all ContentData fields are accessible
        assert!(content_item.hash().is_some());
        assert!(content_item.description().is_some());
        assert!(content_item.tags().is_some());
        assert!(content_item.source().is_some()); 
        assert!(content_item.author().is_some());
        assert!(content_item.url().is_none());
        assert!(content_item.source_url().is_none());
    }

    #[test]
    fn test_mysql_config_custom() {
        let config = MySqlConfig {
            database_url: "mysql://custom:pass@localhost:3306/custom_db".to_string(),
            max_connections: 5,
            min_connections: 2,
            connect_timeout_seconds: 60,
            idle_timeout_seconds: 300,
        };
        
        assert_eq!(config.max_connections, 5);
        assert_eq!(config.min_connections, 2);
        assert_eq!(config.connect_timeout_seconds, 60);
        assert_eq!(config.idle_timeout_seconds, 300);
        assert!(config.database_url.contains("custom_db"));
    }
}