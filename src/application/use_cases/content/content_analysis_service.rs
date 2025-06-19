use serde_json::Value;
use crate::core::platform::container::content::ContentItem;

/// Trait for content analysis services
pub trait ContentAnalysisService {
    fn analyze_content(&self, content: &ContentItem) -> Result<Value, String>;
}

/// Use case for analyzing content
pub struct AnalyzeContent<T: ContentAnalysisService> {
    service: T,
}

impl<T: ContentAnalysisService> AnalyzeContent<T> {
    pub fn new(service: T) -> Self {
        Self { service }
    }

    pub fn execute(&self, content: &ContentItem) -> Result<Value, String> {
        self.service.analyze_content(content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::platform::container::content::{ContentItem, ContentType, TextContent};
    use serde_json::Value;

    struct MockContentAnalysisService;

    impl ContentAnalysisService for MockContentAnalysisService {
        fn analyze_content(&self, _content: &ContentItem) -> Result<Value, String> {
            Ok(Value::String("Test Summary".to_string()))
        }
    }

    #[test]
    fn test_analyze_content() {
        let service = MockContentAnalysisService;
        let use_case = AnalyzeContent::new(service);
        
        // Create a proper ContentItem for testing
        let text_content = TextContent::new(
            None, 
            Some("Mock Content".to_string())
        ).expect("Failed to create text content");
        
        let content = ContentItem::new_with_title(
            ContentType::Text(text_content),
            "Mock Title".to_string(),
        ).expect("Failed to create content item");
        
        let result = use_case.execute(&content);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Value::String("Test Summary".to_string()));
    }
}