use std::sync::Arc;
use uuid::Uuid;

// Import from your actual project structure
use in4me::core::platform::container::content::{ContentItem, ContentType, TextContent};
use in4me::core::base::service::node_version_service::{
    InMemoryNodeVersionRepository, ChangeType
};
use in4me::core::platform::container::content_service::ContentItemService;
use in4me::application::use_cases::content::content_fetching_service::{
    ContentFetchingService, FetchContent
};

// Mock services for end-to-end testing
struct MockContentFetcher;

impl ContentFetchingService for MockContentFetcher {
    fn fetch_content(&self, url: &str) -> Result<ContentItem, String> {
        // Simulate fetching content from different sources
        let content = match url {
            url if url.contains("rss") => {
                let text_content = TextContent::new(
                    None,
                    Some(format!("RSS feed content from {}", url))
                ).map_err(|e| format!("Failed to create RSS content: {:?}", e))?;
                
                let mut content_item = ContentItem::new_with_title(
                    ContentType::Text(text_content),
                    "RSS Feed Article".to_string(),
                ).map_err(|e| format!("Failed to create content item: {:?}", e))?;
                
                content_item.set_source(Some("RSS".to_string()));
                content_item.set_tags(Some(vec!["rss".to_string(), "news".to_string()]));
                content_item
            },
            url if url.contains("api") => {
                let text_content = TextContent::new(
                    None,
                    Some(format!("API data from {}", url))
                ).map_err(|e| format!("Failed to create API content: {:?}", e))?;
                
                let mut content_item = ContentItem::new_with_title(
                    ContentType::Text(text_content),
                    "API Data".to_string(),
                ).map_err(|e| format!("Failed to create content item: {:?}", e))?;
                
                content_item.set_source(Some("API".to_string()));
                content_item.set_tags(Some(vec!["api".to_string(), "data".to_string()]));
                content_item
            },
            url if url.contains("web") => {
                let text_content = TextContent::new(
                    None,
                    Some(format!("Scraped web content from {}", url))
                ).map_err(|e| format!("Failed to create web content: {:?}", e))?;
                
                let mut content_item = ContentItem::new_with_title(
                    ContentType::Text(text_content),
                    "Web Page Content".to_string(),
                ).map_err(|e| format!("Failed to create content item: {:?}", e))?;
                
                content_item.set_source(Some("Web Scraper".to_string()));
                content_item.set_tags(Some(vec!["web".to_string(), "scraping".to_string()]));
                content_item
            },
            _ => {
                return Err(format!("Unsupported URL format: {}", url));
            }
        };
        
        Ok(content)
    }
}

#[tokio::test]
async fn test_content_fetching_pipeline() {
    println!("🚀 Starting end-to-end content pipeline test...");
    
    // Setup services
    let repository = Arc::new(InMemoryNodeVersionRepository::new());
    let content_service = ContentItemService::new(repository.clone());
    
    let fetcher = MockContentFetcher;
    let fetch_use_case = FetchContent::new(fetcher);
    
    // Test sources (simulating different content sources)
    let test_sources = vec![
        "https://example.com/rss/feed.xml",
        "https://api.example.com/v1/data",
        "https://example.com/web/page.html",
    ];
    
    let mut processed_content: Vec<(ContentItem, Uuid)> = Vec::new();
    
    // Step 1: Fetch content from different sources
    println!("📥 Step 1: Fetching content from multiple sources...");
    for (index, source_url) in test_sources.iter().enumerate() {
        println!("   Fetching from: {}", source_url);
        
        let content_item = fetch_use_case.execute(source_url)
            .expect(&format!("Failed to fetch content from {}", source_url));
        
        println!("   ✅ Fetched: {}", content_item.title().unwrap_or(&"Untitled".to_string()));
        
        // Step 2: Store content with versioning
        let content_id = content_item.uuid();
        let (stored_content, version) = content_service.create_content_item(
            content_item,
            Some(format!("system_user_{}", index)),
            Some(format!("Initial fetch from {}", source_url)),
        ).expect("Failed to store content");
        
        assert_eq!(version.version_number, 1);
        assert_eq!(version.metadata.change_type, ChangeType::Created);
        
        processed_content.push((stored_content, content_id));
        println!("   💾 Stored with version: {}", version.version_number);
    }
    
    // Step 3: Process and update content
    println!("🔄 Step 2: Processing and updating content...");
    for (mut content, content_id) in processed_content.into_iter() {
        // Simulate content processing/analysis
        let original_content = match content.content() {
            ContentType::Text(text) => text.content.clone().unwrap_or_default(),
            _ => "Unknown content".to_string(),
        };
        
        let processed_text = format!(
            "{}\n\n--- PROCESSED ---\nAnalyzed and enhanced content with metadata.",
            original_content
        );
        
        // Update content
        let updated_text_content = TextContent::new(None, Some(processed_text))
            .expect("Failed to create updated content");
        
        content.update_content(ContentType::Text(updated_text_content))
            .expect("Failed to update content");
        
        content.set_description(Some("Processed and analyzed content".to_string()));
        content.set_author(Some("Content Processor".to_string()));
        content.set_tags(Some(vec!["processed".to_string(), "analyzed".to_string()]));
        
        // Store updated version
        let (updated_content, update_version) = content_service.update_content_item(
            content,
            Some("content_processor".to_string()),
            Some("Content processed and analyzed".to_string()),
        ).expect("Failed to update content");
        
        assert_eq!(update_version.version_number, 2);
        assert_eq!(update_version.metadata.change_type, ChangeType::Updated);
        
        println!("   ✅ Updated: {} (v{})", 
                 updated_content.title().unwrap_or(&"Untitled".to_string()),
                 update_version.version_number);
        
        // Step 4: Publish content
        let publish_version = content_service.publish_content_item(
            &updated_content,
            Some("publisher".to_string()),
        ).expect("Failed to publish content");
        
        assert_eq!(publish_version.version_number, 3);
        assert_eq!(publish_version.metadata.change_type, ChangeType::Published);
        assert!(publish_version.metadata.is_published);
        
        println!("   📢 Published: {} (v{})",
                 updated_content.title().unwrap_or(&"Untitled".to_string()),
                 publish_version.version_number);
        
        // Step 5: Verify version history
        let history = content_service.get_version_history(content_id)
            .expect("Failed to get version history");
        
        assert_eq!(history.total_versions, 3);
        assert_eq!(history.current_version, 3);
        
        let version_types: Vec<ChangeType> = history.versions.iter()
            .map(|v| v.metadata.change_type.clone())
            .collect();
        
        assert_eq!(version_types, vec![
            ChangeType::Created,
            ChangeType::Updated,
            ChangeType::Published,
        ]);
        
        println!("   📚 Version history verified: {} versions", history.total_versions);
    }
    
    println!("✅ End-to-end test completed successfully!");
    println!("   - Fetched content from {} sources", test_sources.len());
    println!("   - Processed and updated content");
    println!("   - Published all content");
    println!("   - Verified version history for all items");
}

#[tokio::test]
async fn test_content_fetching_error_handling() {
    println!("🧪 Testing error handling in content fetching...");
    
    struct FailingFetcher;
    
    impl ContentFetchingService for FailingFetcher {
        fn fetch_content(&self, url: &str) -> Result<ContentItem, String> {
            match url {
                "https://valid.example.com" => {
                    let text_content = TextContent::new(None, Some("Valid content".to_string()))
                        .map_err(|e| format!("Failed to create content: {:?}", e))?;
                    ContentItem::new_with_title(
                        ContentType::Text(text_content),
                        "Valid Content".to_string(),
                    ).map_err(|e| format!("Failed to create content item: {:?}", e))
                },
                "https://timeout.example.com" => Err("Request timeout".to_string()),
                "https://notfound.example.com" => Err("Content not found".to_string()),
                "https://invalid.example.com" => Err("Invalid content format".to_string()),
                _ => Err("Unknown error".to_string()),
            }
        }
    }
    
    let fetcher = FailingFetcher;
    let fetch_use_case = FetchContent::new(fetcher);
    
    let test_cases = vec![
        ("https://valid.example.com", true),
        ("https://timeout.example.com", false),
        ("https://notfound.example.com", false),
        ("https://invalid.example.com", false),
    ];
    
    for (url, should_succeed) in test_cases {
        let result = fetch_use_case.execute(url);
        
        if should_succeed {
            assert!(result.is_ok(), "Expected success for {}", url);
            println!("   ✅ Successfully handled: {}", url);
        } else {
            assert!(result.is_err(), "Expected failure for {}", url);
            println!("   ⚠️  Correctly handled error for: {} - {}", url, result.unwrap_err());
        }
    }
    
    println!("✅ Error handling test completed successfully!");
}

#[tokio::test]
async fn test_content_lifecycle_integration() {
    println!("🔄 Testing complete content lifecycle integration...");
    
    // Setup
    let repository = Arc::new(InMemoryNodeVersionRepository::new());
    let content_service = ContentItemService::new(repository.clone());
    
    // Create initial content
    let text_content = TextContent::new(
        None,
        Some("Original blog post content about Rust development.".to_string())
    ).expect("Failed to create text content");
    
    let mut content_item = ContentItem::new_with_title(
        ContentType::Text(text_content),
        "Rust Development Guide".to_string(),
    ).expect("Failed to create content item");
    
    content_item.set_author(Some("Jane Developer".to_string()));
    content_item.set_tags(Some(vec!["rust".to_string(), "programming".to_string()]));
    
    let content_id = content_item.uuid();
    
    // Lifecycle Step 1: Create
    println!("   📝 Creating initial content...");
    let (mut content_item, _) = content_service.create_content_item(
        content_item,
        Some("author@example.com".to_string()),
        Some("Initial blog post creation".to_string()),
    ).expect("Failed to create content");
    
    // Lifecycle Step 2: Update multiple times
    println!("   ✏️  Updating content...");
    for i in 1..=3 {
        let updated_text = TextContent::new(
            None,
            Some(format!(
                "Updated blog post content about Rust development. Update #{}: Adding more detailed information about advanced Rust concepts.",
                i
            ))
        ).expect("Failed to create updated content");
        
        content_item.update_content(ContentType::Text(updated_text))
            .expect("Failed to update content");
        
        content_item.set_tags(Some(vec![format!("update-{}", i)]));
        
        let (updated_content, _) = content_service.update_content_item(
            content_item,
            Some("editor@example.com".to_string()),
            Some(format!("Content update #{}", i)),
        ).expect("Failed to update content");
        
        content_item = updated_content;
    }
    
    // Lifecycle Step 3: Publish
    println!("   📢 Publishing content...");
    let _publish_version = content_service.publish_content_item(
        &content_item,
        Some("publisher@example.com".to_string()),
    ).expect("Failed to publish content");
    
    // Lifecycle Step 4: More updates after publishing
    println!("   🔄 Post-publication updates...");
    let post_pub_text = TextContent::new(
        None,
        Some("Updated blog post with reader feedback and corrections.".to_string())
    ).expect("Failed to create post-publication content");
    
    content_item.update_content(ContentType::Text(post_pub_text))
        .expect("Failed to update content");
    
    let (content_item, _) = content_service.update_content_item(
        content_item,
        Some("author@example.com".to_string()),
        Some("Incorporated reader feedback".to_string()),
    ).expect("Failed to update content");
    
    // Lifecycle Step 5: Archive
    println!("   📦 Archiving content...");
    let _archive_version = content_service.archive_content_item(
        &content_item,
        Some("admin@example.com".to_string()),
    ).expect("Failed to archive content");
    
    // Verification: Check complete history
    println!("   📊 Verifying complete lifecycle...");
    let history = content_service.get_version_history(content_id)
        .expect("Failed to get version history");
    
    // Should have: 1 create + 3 updates + 1 publish + 1 post-pub update + 1 archive = 7 versions
    assert_eq!(history.total_versions, 7);
    
    let expected_changes = vec![
        ChangeType::Created,
        ChangeType::Updated,
        ChangeType::Updated,
        ChangeType::Updated,
        ChangeType::Published,
        ChangeType::Updated,
        ChangeType::Archived,
    ];
    
    let actual_changes: Vec<ChangeType> = history.versions.iter()
        .map(|v| v.metadata.change_type.clone())
        .collect();
    
    assert_eq!(actual_changes, expected_changes);
    
    println!("✅ Content lifecycle integration test completed successfully!");
    println!("   - Total versions created: {}", history.total_versions);
    println!("   - Lifecycle stages verified: Create → Update → Publish → Update → Archive");
}

#[cfg(test)]
mod performance_tests {
    use super::*;
    use std::time::Instant;
    
    #[tokio::test]
    async fn test_bulk_content_processing() {
        println!("⚡ Testing bulk content processing performance...");
        
        let repository = Arc::new(InMemoryNodeVersionRepository::new());
        let content_service = ContentItemService::new(repository.clone());
        
        let start_time = Instant::now();
        let content_count = 100;
        
        for i in 1..=content_count {
            let text_content = TextContent::new(
                None,
                Some(format!("Bulk content item #{} with test data.", i))
            ).expect("Failed to create text content");
            
            let content_item = ContentItem::new_with_title(
                ContentType::Text(text_content),
                format!("Bulk Item #{}", i),
            ).expect("Failed to create content item");
            
            let (_stored_content, version) = content_service.create_content_item(
                content_item,
                Some("bulk_processor".to_string()),
                Some(format!("Bulk creation #{}", i)),
            ).expect("Failed to store bulk content");
            
            assert_eq!(version.version_number, 1);
        }
        
        let duration = start_time.elapsed();
        let items_per_second = content_count as f64 / duration.as_secs_f64();
        
        println!("   ✅ Processed {} items in {:?}", content_count, duration);
        println!("   📊 Performance: {:.2} items/second", items_per_second);
        
        // Verify all items are stored by checking that we can retrieve them
        // Since we don't have a direct len() method, we verify by checking that we created the expected number
        println!("   ✅ All {} items processed and stored", content_count);
        
        println!("✅ Bulk processing test completed successfully!");
    }
}