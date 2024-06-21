// src/domain/services/aggregate_content.rs
use serde_json::Value;
use crate::domain::services::content_aggregation_service::ContentAggregationService;

pub struct AggregateContent<T: ContentAggregationService> {
    service: T,
}

impl<T: ContentAggregationService> AggregateContent<T> {
    pub fn new(service: T) -> Self {
        Self { service }
    }

    pub fn execute(&self, data: Vec<Value>) -> Value {
        self.service.aggregate_content(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockContentAggregationService;

    impl ContentAggregationService for MockContentAggregationService {
        fn aggregate_content(&self, data: Vec<Value>) -> Value {
            Value::Array(data)
        }
    }

    #[test]
    fn test_aggregate_content() {
        let service = MockContentAggregationService;
        let use_case = AggregateContent::new(service);
        let data = vec![Value::String("Test Data".to_string())];
        let result = use_case.execute(data);
        assert_eq!(result, Value::Array(vec![Value::String("Test Data".to_string())]));
    }
}
