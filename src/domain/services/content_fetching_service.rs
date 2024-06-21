// src/domain/services/content_fetching_service.rs
use crate::entities::content_list::ContentList;
use crate::domain::entities::content_item::ContentItem;

pub trait ContentFetchingService {
    fn fetch_content(&self, location: &str) -> Result<ContentItem, String>;
}

pub trait ContentListFetchingService {
    fn fetch_content_list(&self, directory: &str) -> Result<ContentList, String>;
}