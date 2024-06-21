// src/adapters/primary/simple_content_filter.rs
use crate::domain::entities::content_list::ContentList;
use crate::domain::entities::content_item::ContentItem;
use crate::domain::services::content_filter_service::{ContentItemFilterService, ContentListFilterService};

pub struct ContentFilter;

impl ContentItemFilterService for ContentFilter {
    fn filter_content(&self, content: &ContentItem, keyword: &str) -> bool {
        content..contains(keyword)
    }
}

impl ContentListFilterService for ContentFilter {
    fn filter_content_list(&self, content_list: &ContentList, keyword: &str) -> Vec<ContentList> {
        content_list
            .list_items
            .iter()
            .filter(|content| self.filter_content(content, keyword))
            .map(|content| ContentList {
                uuid: todo!(),
                name: todo!(),
                url: todo!(),
                created: todo!(),
                modified: todo!(),
                list_items: todo!(),
            })
            .collect()
    }
}

// src/adapters/primary/simple_content_filter.rs (unit test)
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_content() {
        let content = ContentItem {
            uuid: todo!(),
            created: todo!(),
            modified: todo!(),
            source_data: todo!(),
            content: todo!(),
        };

        let filter = ContentFilter;
        assert!(filter.filter_content(&content, "world"));
        assert!(!filter.filter_content(&content, "rust"));
    }
}
