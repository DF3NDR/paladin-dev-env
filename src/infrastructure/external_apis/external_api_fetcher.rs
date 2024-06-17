use crate::domain::entities::normalized_data::NormalizedData;
use crate::domain::services::content_fetching_service::ContentFetchingService;

pub struct ExternalApiFetcher;

impl ContentFetchingService for ExternalApiFetcher {
    fn fetch_content(&self, url: &str) -> Vec<NormalizedData> {
        // Implementation here
        vec![]
    }
}
