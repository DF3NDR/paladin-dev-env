use serde_json::Value;
use crate::domain::services::content_analysis_service::ContentAnalysisService;
use crate::domain::entities::normalized_data::NormalizedData;

pub struct AnalyzeContentUseCase<T: ContentAnalysisService> {
    service: T,
}

impl<T: ContentAnalysisService> AnalyzeContentUseCase<T> {
    pub fn new(service: T) -> Self {
        Self { service }
    }

    pub fn execute(&self, content: &NormalizedData) -> Result<Value, String> {
        self.service.analyze_content(content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::services::content_analysis_service::ContentAnalysisService;
    use crate::domain::entities::normalized_data::NormalizedData;
    use serde_json::Value;

    struct MockContentAnalysisService;

    impl ContentAnalysisService for MockContentAnalysisService {
        fn analyze_content(&self, _content: &NormalizedData) -> Result<Value, String> {
            Ok(Value::String("Test Summary".to_string()))
        }
    }

    #[test]
    fn test_analyze_content() {
        let service = MockContentAnalysisService;
        let use_case = AnalyzeContentUseCase::new(service);
        let content = NormalizedData {
            title: "Test Title".to_string(),
            link: "http://example.com".to_string(),
            description: "Test Description".to_string(),
        };
        let result = use_case.execute(&content);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Value::String("Test Summary".to_string()));
    }
}

