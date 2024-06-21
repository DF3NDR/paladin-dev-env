use serde_json::Value;
use crate::entities::content_item::ContentItem;

pub trait ContentAnalysisService {
    // Todo - Result and Error should be a custom types
    fn analyze_content(&self, content: &ContentItem) -> Result<Value, String>;
}
