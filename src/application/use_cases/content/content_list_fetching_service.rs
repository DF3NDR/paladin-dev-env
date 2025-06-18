use crate::core::platform::container::content_list::ContentList;

pub trait ContentListFetchingService {
    fn fetch_content_list(&self, url: &str) -> Result<ContentList, String>;
}

pub struct FetchContentList<T: ContentListFetchingService> {
    service: T,
}

impl<T: ContentListFetchingService> FetchContentList<T> {
    pub fn new(service: T) -> Self {
        Self { service }
    }

    pub fn execute(&self, directory: &str) -> Result<ContentList, String> {
        self.service.fetch_content_list(directory)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;
    use url::Url;
    use chrono::Utc;

    struct MockContentListFetchingService;

    impl ContentListFetchingService for MockContentListFetchingService {
        fn fetch_content_list(&self, _url: &str) -> Result<ContentList, String> {
            let test_url = Url::parse("https://example.com/test-list")
                .map_err(|e| format!("Failed to parse URL: {}", e))?;
            
            let mut content_list = ContentList::with_name("test.txt".to_string());
            content_list.set_url(Some(test_url));
            
            Ok(content_list)
        }
    }

    #[test]
    fn test_fetch_content_list() {
        let service = MockContentListFetchingService;
        let use_case = FetchContentList::new(service);
        let result = use_case.execute("dummy_directory");

        assert!(result.is_ok());
        let content_list = result.unwrap();
        assert_eq!(content_list.name, Some("test.txt".to_string()));
        assert_eq!(content_list.len(), 0);
        assert!(content_list.uuid != Uuid::nil());
    }

    #[test]
    fn test_fetch_content_list_with_error() {
        struct FailingMockService;
        
        impl ContentListFetchingService for FailingMockService {
            fn fetch_content_list(&self, _url: &str) -> Result<ContentList, String> {
                Err("Failed to fetch content list".to_string())
            }
        }

        let service = FailingMockService;
        let use_case = FetchContentList::new(service);
        let result = use_case.execute("dummy_directory");

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Failed to fetch content list");
    }

    #[test]
    fn test_content_list_properties() {
        let service = MockContentListFetchingService;
        let use_case = FetchContentList::new(service);
        let result = use_case.execute("dummy_directory");

        assert!(result.is_ok());
        let content_list = result.unwrap();
        
        // Test that the URL is valid
        assert_eq!(content_list.url.as_ref().unwrap().as_str(), "https://example.com/test-list");
        
        // Test that timestamps are reasonable (within the last minute)
        let now = Utc::now();
        let time_diff = now.signed_duration_since(content_list.created);
        assert!(time_diff.num_seconds() < 60); // Created within last minute
        
        // Test that created and modified are close to each other
        let creation_diff = content_list.modified.signed_duration_since(content_list.created);
        assert!(creation_diff.num_milliseconds() < 1000); // Within 1 second
    }
}