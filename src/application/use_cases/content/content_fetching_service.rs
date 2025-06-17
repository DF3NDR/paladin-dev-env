use crate::core::platform::container::content::ContentItem;

pub trait ContentFetchingService {
    fn fetch_content(&self, url: &str) -> Result<ContentItem, String>;
}

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
                content: todo!(),
                url: todo!(),
                hash: todo!(),
                source_id: todo!(),
                source_url: todo!(),
                title: todo!(),
                description: todo!(),
                tags: todo!(),
                source: todo!(),
                author: todo!(),
                pub_date: todo!(),
                mod_date: todo!(),
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
