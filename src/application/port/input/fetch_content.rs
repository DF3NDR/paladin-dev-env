// src/application/use_cases/fetch_content.rs
use crate::domain::services::content_fetching_service::ContentFetchingService;
use crate::domain::entities::content_item::ContentItem;

pub struct FetchContent<T: ContentFetchingService> {
    service: T,
}

impl<T: ContentFetchingService> FetchContent<T> {
    pub fn new(service: T) -> Self {
        Self { service }
    }

    pub fn execute(&self, url: &str) -> Result<ContentItem, String> {
        self.service.fetch_content(url)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockContentFetchingService;

    impl ContentFetchingService for MockContentFetchingService {
        fn fetch_content(&self, _url: &str) -> Result<ContentItem, String> {
            Ok(ContentItem {
                uuid: todo!(),
                created: todo!(),
                modified: todo!(),
                source_data: todo!(),
                content: todo!(),
            })
        }
    }

    #[test]
    fn test_fetch_content() {
        let service = MockContentFetchingService;
        let use_case = FetchContent::new(service);
        let result = use_case.execute("test.txt");
        assert!(result.is_ok());
        let content = result.unwrap();
        // assert_eq!(content.metadata., "test.txt"); // ToDo - Implement Mock ContentItem validation
        // assert_eq!(content.body, "Test data");
    }
}
