use serde_json::Value;
use crate::domain::services::content_aggregation_service::ContentAggregationService;

pub struct AggregateContentUseCase<T: ContentAggregationService> {
    service: T,
}

impl<T: ContentAggregationService> AggregateContentUseCase<T> {
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
    use serde_json::Value;
    use crate::domain::services::content_aggregation_service::ContentAggregationService;

    struct MockContentAggregationService;

    impl ContentAggregationService for MockContentAggregationService {
        fn aggregate_content(&self, data: Vec<Value>) -> Value {
            Value::Array(data)
        }
    }

    #[test]
    fn test_aggregate_content() {
        let service = MockContentAggregationService;
        let use_case = AggregateContentUseCase::new(service);
        let data = vec![Value::String("Test Data".to_string())];
        let result = use_case.execute(data);
        assert_eq!(result, Value::Array(vec![Value::String("Test Data".to_string())]));
    }
}
