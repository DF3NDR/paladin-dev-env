use std::vec;

// src/domain/services/content_filter_service.rs
use crate::domain::entities::content_item::ContentItem;
use crate::domain::entities::content_list::ContentList;

pub trait ContentItemFilterService {
    fn filter_content(&self, content: &ContentItem, keyword: &str) -> bool;
}

// This is a trait that defines the behavior of a service that filters a list of 
// content items based on a keyword.
pub trait ContentListFilterService {
    fn filter_content_list(&self, content_list: &ContentList, keywords: &str) -> Vec<ContentList>;
}
