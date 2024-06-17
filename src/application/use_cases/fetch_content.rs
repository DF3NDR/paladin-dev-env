use crate::domain::services::content_fetching_service::ContentFetchingService;

pub struct FetchContentUseCase<T: ContentFetchingService> {
    service: T,
}

impl<T: ContentFetchingService> FetchContentUseCase<T> {
    pub fn new(service: T) -> Self {
        Self { service }
    }

    pub fn execute(&self, url: &str) {
        self.service.fetch_content(url);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::normalized_data::NormalizedData;
    use crate::domain::services::content_fetching_service::ContentFetchingService;

    struct MockContentFetchingService;

    impl ContentFetchingService for MockContentFetchingService {
        fn fetch_content(&self, _url: &str) -> Vec<NormalizedData> {
            vec![
                NormalizedData {
                    title: "Test Title".to_string(),
                    link: "http://example.com".to_string(),
                    description: "Test Description".to_string(),
                }
            ]
        }
    }

    #[test]
    fn test_fetch_content() {
        let service = MockContentFetchingService;
        let use_case = FetchContentUseCase::new(service);
        let result = use_case.execute("http://example.com");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].title, "Test Title");
    }
}
