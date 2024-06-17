use serde_json::Value;
use crate::entities::normalized_data::NormalizedData;

pub trait ContentAnalysisService {
    fn analyze_content(&self, content: &NormalizedData) -> Result<Value, String>;
}
