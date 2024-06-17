use serde_json::Value;
use crate::domain::services::content_aggregation_service::ContentAggregationService;

pub struct ContentAggregator;

impl ContentAggregationService for ContentAggregator {
    fn aggregate_content(&self, data: Vec<Value>) -> Value {
        // Implementation here
        Value::Array(data)
    }
}
