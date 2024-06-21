
use serde_json::Value;
use crate::domain::services::content_analysis_service::ContentAnalysisService;
use crate::domain::entities::content_item::ContentItem;

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
    use crate::domain::services::content_analysis_service::ContentAnalysisService;
    use crate::domain::entities::content_item::ContentItem;
    use chrono::{DateTime, Utc};
    use serde_json::Value;
    use uuid::Uuid;

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
        let content = ContentItem {
            uuid: Uuid::new_v4(),
            created: Utc::now(),
            modified: Utc::now(),
            source_data: SourceData::new("Mock Source".to_string(), "Mock URL".to_string());
            content: crate::entities::content_item::ContentType::Text::new("Mock Content".to_string()),
        };
        let result = use_case.execute(&content);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Value::String("Test Summary".to_string())); // ToDo - Implement Mock ContentItem validation
    }
}
