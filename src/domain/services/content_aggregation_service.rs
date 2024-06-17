use serde_json::Value;

pub trait ContentAggregationService {
    fn aggregate_content(&self, data: Vec<Value>) -> Value;
}
