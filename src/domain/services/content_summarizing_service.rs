// src/domain/services/summarized_content_service.rs
use crate::domain::entities::content_item::ContentItem;

pub trait ContentSummarizingService {
    fn summarize_content(&self, content: &ContentItem, length: u8) -> Option<String>;
}
