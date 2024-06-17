use crate::entities::normalized_data::NormalizedData;

pub trait ContentFetchingService {
    fn fetch_content(&self, url: &str) -> Vec<NormalizedData>;
}
