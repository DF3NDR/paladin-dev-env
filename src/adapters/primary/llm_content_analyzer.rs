use serde_json::Value;
use crate::domain::entities::normalized_data::NormalizedData;
use crate::domain::services::content_analysis_service::ContentAnalysisService;

pub struct LlmContentAnalyzer;

impl ContentAnalysisService for LlmContentAnalyzer {
    fn analyze_content(&self, content: &NormalizedData) -> Result<Value, String> {
        // Implementation here
        Ok(Value::Null)
    }
}
