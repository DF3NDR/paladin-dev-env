use crate::domain::entities::normalized_data::NormalizedData;
use crate::domain::services::content_fetching_service::ContentFetchingService;

pub struct HttpContentFetcher;

impl ContentFetchingService for HttpContentFetcher {
    fn fetch_content(&self, url: &str) -> Vec<NormalizedData> {
        // Implementation here
        vec![]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::services::content_fetching_service::ContentFetchingService;
    use crate::domain::entities::normalized_data::NormalizedData;

    #[test]
    fn test_fetch_content() {
        let fetcher = HttpContentFetcher;
//        let result = fetcher.fetch_content("http://example.com");
        // Validate the result
    }
}
