/*
Content Aggregator Service Use Case

This use case offers a service for aggregating content. 

It takes a list of content items as input and returns a single aggregated content list.
*/
use serde_json::Value;
use crate::core::platform::container::content_list::ContentList;

pub trait ContentListService {
    fn fetch_content_list(&self, url: &str) -> Result<ContentList, String>;
    fn aggregate_content(&self, data: Vec<Value>) -> Value;
}

pub struct AggregateContent<T: ContentListService> {
    service: T,
}

impl<T: ContentListService> AggregateContent<T> {
    pub fn new(service: T) -> Self {
        Self { service }
    }

    pub fn execute(&self, data: Vec<Value>) -> Value {
        self.service.aggregate_content(data)
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;

//     struct MockContentAggregationService;

//     impl ContentAggregationService for MockContentAggregationService {
//         fn aggregate_content(&self, data: Vec<Value>) -> Value {
//             Value::Array(data)
//         }
//     }

//     #[test]
//     fn test_aggregate_content() {
//         let service = MockContentAggregationService;
//         let use_case = AggregateContent::new(service);
//         let data = vec![Value::String("Test Data".to_string())];
//         let result = use_case.execute(data);
//         assert_eq!(result, Value::Array(vec![Value::String("Test Data".to_string())]));
//     }
// }
