use crate::core::platform::container::content::ContentItem;

pub trait ContentFetchingService {
    fn fetch_content(&self, url: &str) -> Result<ContentItem, String>;
}

pub struct FetchContent<T: ContentFetchingService> {
    service: T,
}

impl<T: ContentFetchingService> FetchContent<T> {
    pub fn new(service: T) -> Self {
        Self { service }
    }

    pub fn execute(&self, url: &str) -> Result<ContentItem, String> {
        self.service.fetch_content(url)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::platform::container::content::{ContentType, TextContent};
    use uuid::Uuid;
    use url::Url;
    use chrono::Utc;

    struct MockContentFetchingService;

    impl ContentFetchingService for MockContentFetchingService {
        fn fetch_content(&self, _url: &str) -> Result<ContentItem, String> {
            // Create a proper TextContent first
            let text_content = TextContent::new(None, Some("test content".to_string()))
                .map_err(|e| format!("Failed to create text content: {:?}", e))?;
            
            // Create ContentType
            let content_type = ContentType::Text(text_content);
            
            // Create ContentItem using the new() method
            let mut content_item = ContentItem::new(content_type)
                .map_err(|e| format!("Failed to create content item: {:?}", e))?;
            
            // Set optional fields that the test expects
            content_item.url = Some(Url::parse("https://example.com/test")
                .map_err(|e| format!("Invalid URL: {}", e))?);
            content_item.hash = Some("test-hash".to_string());
            content_item.source_id = Some("test-source-id".to_string());
            content_item.source_url = Some(Url::parse("https://example.com/source")
                .map_err(|e| format!("Invalid source URL: {}", e))?);
            content_item.title = Some("test title".to_string());
            content_item.description = Some("test description".to_string());
            content_item.tags = Some(vec!["test-tag".to_string()]);
            content_item.source = Some("test source".to_string());
            content_item.author = Some("test author".to_string());
            content_item.pub_date = Some(Utc::now());
            content_item.mod_date = Some(Utc::now());
            
            Ok(content_item)
        }
    }

    #[test]
    fn test_fetch_content() {
        let service = MockContentFetchingService;
        let use_case = FetchContent::new(service);
        let result = use_case.execute("https://example.com/test.txt");
        
        assert!(result.is_ok());
        let _content = result.unwrap();
        
        // Now we can properly test the content
        // assert_eq!(content.title, Some("test title".to_string()));
        // assert!(content.uuid != Uuid::nil());
        // assert!(matches!(content.content, ContentType::Text(_)));
    }

    #[test]
    fn test_fetch_content_with_error() {
        struct FailingMockService;
        
        impl ContentFetchingService for FailingMockService {
            fn fetch_content(&self, _url: &str) -> Result<ContentItem, String> {
                Err("Failed to fetch content".to_string())
            }
        }

        let service = FailingMockService;
        let use_case = FetchContent::new(service);
        let result = use_case.execute("https://example.com/nonexistent.txt");

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Failed to fetch content");
    }

    #[test]
    fn test_fetch_different_content_types() {
        use crate::core::platform::container::content::{VideoContent, AudioContent, ImageContent};

        struct MultiTypeService;
        
        impl ContentFetchingService for MultiTypeService {
            fn fetch_content(&self, url: &str) -> Result<ContentItem, String> {
                let content_type = if url.contains("video") {
                    let video_content = VideoContent::new(None, 3600)
                        .map_err(|e| format!("Failed to create video content: {:?}", e))?;
                    ContentType::Video(video_content)
                } else if url.contains("audio") {
                    let audio_content = AudioContent::new(None, 180)
                        .map_err(|e| format!("Failed to create audio content: {:?}", e))?;
                    ContentType::Audio(audio_content)
                } else if url.contains("image") {
                    let image_content = ImageContent::new(None, (1920, 1080))
                        .map_err(|e| format!("Failed to create image content: {:?}", e))?;
                    ContentType::Image(image_content)
                } else {
                    let text_content = TextContent::new(None, Some("default text".to_string()))
                        .map_err(|e| format!("Failed to create text content: {:?}", e))?;
                    ContentType::Text(text_content)
                };

                ContentItem::new(content_type)
                    .map_err(|e| format!("Failed to create content item: {:?}", e))
            }
        }

        let service = MultiTypeService;
        let use_case = FetchContent::new(service);

        // Test different content types
        let video_result = use_case.execute("https://example.com/video.mp4");
        assert!(video_result.is_ok());
        assert!(matches!(video_result.unwrap().content, ContentType::Video(_)));

        let audio_result = use_case.execute("https://example.com/audio.mp3");
        assert!(audio_result.is_ok());
        assert!(matches!(audio_result.unwrap().content, ContentType::Audio(_)));

        let image_result = use_case.execute("https://example.com/image.jpg");
        assert!(image_result.is_ok());
        assert!(matches!(image_result.unwrap().content, ContentType::Image(_)));

        let text_result = use_case.execute("https://example.com/text.txt");
        assert!(text_result.is_ok());
        assert!(matches!(text_result.unwrap().content, ContentType::Text(_)));
    }

    #[test]
    fn test_content_item_properties() {
        let service = MockContentFetchingService;
        let use_case = FetchContent::new(service);
        let result = use_case.execute("https://example.com/test.txt");

        assert!(result.is_ok());
        let content = result.unwrap();

        // Test that required fields are set
        assert!(content.uuid != Uuid::nil());
        assert!(content.created <= Utc::now());
        assert_eq!(content.created, content.modified);
        
        // Test optional fields that were set in the mock
        assert_eq!(content.title, Some("test title".to_string()));
        assert_eq!(content.description, Some("test description".to_string()));
        assert_eq!(content.tags, Some(vec!["test-tag".to_string()]));
        assert_eq!(content.source, Some("test source".to_string()));
        assert_eq!(content.author, Some("test author".to_string()));
        assert!(content.url.is_some());
        assert!(content.source_url.is_some());
        assert!(content.pub_date.is_some());
        assert!(content.mod_date.is_some());
    }
}