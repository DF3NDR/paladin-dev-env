use crate::domain::entities::content_item::ContentItem;
use crate::domain::services::content_summarizing_service::ContentSummarizingService;

pub struct ContentSummarizer;

impl ContentSummarizingService for ContentSummarizer {
    fn summarize_content(&self, content: &ContentItem, length: u8) -> Option<String> {
        // Return the first `length` characters of the content.body.body field.
        content.body.body.as_ref().map(|s| s.chars().take(length.into()).collect())
    }
}

// src/adapters/primary/simple_content_summarizer.rs (unit test)
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_summarize_content() {
        let content = ContentItem {
            uuid: todo!(),
            created: todo!(),
            modified: todo!(),
            source_data: todo!(),
            content: todo!(),
        };

        let summarizer = ContentSummarizer;
        let summary = summarizer.summarize_content(&content, 255);

        assert_eq!(summary, Some("Hello, world!".to_string()));
    }
}
