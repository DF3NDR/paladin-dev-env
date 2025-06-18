use super::content::ContentItem;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use url::Url;
use chrono::prelude::*;
use thiserror::Error;
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ContentList {
    pub uuid: Uuid,
    pub created: DateTime<Utc>,
    pub modified: DateTime<Utc>,
    pub items: Vec<ContentItem>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
    pub source: Option<String>,
    pub url: Option<Url>,
    pub source_url: Option<Url>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ContentListTypes {
    ContentItemsOnly(Vec<ContentItem>),
    ContentListWithMeta(ContentList),
}

impl ContentList {
    pub fn new() -> Self {
        let now = Utc::now();
        Self {
            uuid: Uuid::new_v4(),
            created: now,
            modified: now,
            items: Vec::new(),
            name: None,
            description: None,
            tags: None,
            source: None,
            url: None,
            source_url: None,
        }
    }

    pub fn with_name(name: String) -> Self {
        let mut list = Self::new();
        list.name = Some(name);
        list
    }

    pub fn add_item(&mut self, item: ContentItem) {
        self.items.push(item);
        self.modified = Utc::now();
    }

    pub fn remove_item(&mut self, uuid: Uuid) -> Option<ContentItem> {
        if let Some(pos) = self.items.iter().position(|item| item.uuid() == uuid) {
            self.modified = Utc::now();
            Some(self.items.remove(pos))
        } else {
            None
        }
    }

    pub fn get_item(&self, uuid: Uuid) -> Option<&ContentItem> {
        self.items.iter().find(|item| item.uuid() == uuid)
    }

    pub fn get_item_mut(&mut self, uuid: Uuid) -> Option<&mut ContentItem> {
        self.items.iter_mut().find(|item| item.uuid() == uuid)
    }

    pub fn clear(&mut self) {
        self.items.clear();
        self.modified = Utc::now();
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn items(&self) -> &Vec<ContentItem> {
        &self.items
    }

    pub fn items_mut(&mut self) -> &mut Vec<ContentItem> {
        &mut self.items
    }

    pub fn set_name(&mut self, name: Option<String>) {
        self.name = name;
        self.modified = Utc::now();
    }

    pub fn set_description(&mut self, description: Option<String>) {
        self.description = description;
        self.modified = Utc::now();
    }

    pub fn set_tags(&mut self, tags: Option<Vec<String>>) {
        self.tags = tags;
        self.modified = Utc::now();
    }

    pub fn set_source(&mut self, source: Option<String>) {
        self.source = source;
        self.modified = Utc::now();
    }

    pub fn set_url(&mut self, url: Option<Url>) {
        self.url = url;
        self.modified = Utc::now();
    }

    pub fn set_source_url(&mut self, source_url: Option<Url>) {
        self.source_url = source_url;
        self.modified = Utc::now();
    }

    // Filter methods
    pub fn filter_by_tag(&self, tag: &str) -> Vec<&ContentItem> {
        self.items
            .iter()
            .filter(|item| {
                if let Some(tags) = item.tags() {
                    tags.contains(&tag.to_string())
                } else {
                    false
                }
            })
            .collect()
    }

    pub fn filter_by_author(&self, author: &str) -> Vec<&ContentItem> {
        self.items
            .iter()
            .filter(|item| {
                if let Some(item_author) = item.author() {
                    item_author == author
                } else {
                    false
                }
            })
            .collect()
    }

    pub fn filter_by_source(&self, source: &str) -> Vec<&ContentItem> {
        self.items
            .iter()
            .filter(|item| {
                if let Some(item_source) = item.source() {
                    item_source == source
                } else {
                    false
                }
            })
            .collect()
    }

    pub fn get_unique_urls(&self) -> HashSet<Url> {
        self.items
            .iter()
            .filter_map(|item| item.url().cloned())
            .collect()
    }

    pub fn get_all_tags(&self) -> HashSet<String> {
        let mut all_tags = HashSet::new();
        
        // Add list tags if present
        if let Some(ref tags) = self.tags {
            all_tags.extend(tags.clone());
        }
        
        // Add item tags
        for item in &self.items {
            if let Some(tags) = item.tags() {
                all_tags.extend(tags.clone());
            }
        }
        
        all_tags
    }

    pub fn get_all_authors(&self) -> HashSet<String> {
        self.items
            .iter()
            .filter_map(|item| item.author().cloned())
            .collect()
    }

    pub fn get_all_sources(&self) -> HashSet<String> {
        let mut sources = HashSet::new();
        
        // Add list source if present
        if let Some(ref source) = self.source {
            sources.insert(source.clone());
        }
        
        // Add item sources
        for item in &self.items {
            if let Some(source) = item.source() {
                sources.insert(source.clone());
            }
        }
        
        sources
    }

    // Sort methods
    pub fn sort_by_created(&mut self) {
        self.items.sort_by(|a, b| a.created().cmp(&b.created()));
        self.modified = Utc::now();
    }

    pub fn sort_by_modified(&mut self) {
        self.items.sort_by(|a, b| a.modified().cmp(&b.modified()));
        self.modified = Utc::now();
    }

    pub fn sort_by_title(&mut self) {
        self.items.sort_by(|a, b| {
            match (a.title(), b.title()) {
                (Some(title_a), Some(title_b)) => title_a.cmp(title_b),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            }
        });
        self.modified = Utc::now();
    }

    // Statistics
    pub fn content_type_stats(&self) -> std::collections::HashMap<String, usize> {
        let mut stats = std::collections::HashMap::new();
        
        for item in &self.items {
            let content_type = match item.content() {
                crate::core::platform::container::content::ContentType::Text(_) => "Text",
                crate::core::platform::container::content::ContentType::Video(_) => "Video",
                crate::core::platform::container::content::ContentType::Audio(_) => "Audio",
                crate::core::platform::container::content::ContentType::Image(_) => "Image",
            };
            
            *stats.entry(content_type.to_string()).or_insert(0) += 1;
        }
        
        stats
    }
}

impl Default for ContentList {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Error)]
pub enum ContentListError {
    #[error("Content item not found: {0}")]
    ContentItemNotFound(Uuid),
    #[error("Content list is empty")]
    EmptyContentList,
    #[error("Invalid operation on content list")]
    InvalidOperation,
}

impl From<Vec<ContentItem>> for ContentList {
    fn from(items: Vec<ContentItem>) -> Self {
        let mut list = Self::new();
        list.items = items;
        list.modified = Utc::now();
        list
    }
}

impl From<ContentList> for Vec<ContentItem> {
    fn from(list: ContentList) -> Self {
        list.items
    }
}

impl IntoIterator for ContentList {
    type Item = ContentItem;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.items.into_iter()
    }
}

impl<'a> IntoIterator for &'a ContentList {
    type Item = &'a ContentItem;
    type IntoIter = std::slice::Iter<'a, ContentItem>;

    fn into_iter(self) -> Self::IntoIter {
        self.items.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::platform::container::content::{ContentType, TextContent};

    fn create_test_content_item(title: &str, content: &str) -> ContentItem {
        let text_content = TextContent::new(None, Some(content.to_string())).unwrap();
        ContentItem::new_with_title(
            ContentType::Text(text_content),
            title.to_string(),
        ).unwrap()
    }

    #[test]
    fn test_content_list_creation() {
        let list = ContentList::new();
        
        assert!(list.uuid != Uuid::nil());
        assert_eq!(list.created, list.modified);
        assert!(list.items.is_empty());
        assert_eq!(list.name, None);
    }

    #[test]
    fn test_content_list_with_name() {
        let list = ContentList::with_name("Test List".to_string());
        
        assert_eq!(list.name, Some("Test List".to_string()));
    }

    #[test]
    fn test_add_and_remove_items() {
        let mut list = ContentList::new();
        let item1 = create_test_content_item("Item 1", "Content 1");
        let item1_uuid = item1.uuid();
        
        list.add_item(item1);
        assert_eq!(list.len(), 1);
        
        let removed_item = list.remove_item(item1_uuid);
        assert!(removed_item.is_some());
        assert_eq!(list.len(), 0);
    }

    #[test]
    fn test_filter_by_tag() {
        let mut list = ContentList::new();
        
        let mut item1 = create_test_content_item("Item 1", "Content 1");
        item1.set_tags(Some(vec!["rust".to_string(), "test".to_string()]));
        
        let mut item2 = create_test_content_item("Item 2", "Content 2");
        item2.set_tags(Some(vec!["python".to_string(), "test".to_string()]));
        
        list.add_item(item1);
        list.add_item(item2);
        
        let rust_items = list.filter_by_tag("rust");
        assert_eq!(rust_items.len(), 1);
        
        let test_items = list.filter_by_tag("test");
        assert_eq!(test_items.len(), 2);
    }

    #[test]
    fn test_content_type_stats() {
        let mut list = ContentList::new();
        
        list.add_item(create_test_content_item("Text 1", "Content 1"));
        list.add_item(create_test_content_item("Text 2", "Content 2"));
        
        let stats = list.content_type_stats();
        assert_eq!(stats.get("Text"), Some(&2));
    }

    #[test]
    fn test_sort_by_title() {
        let mut list = ContentList::new();
        
        list.add_item(create_test_content_item("Zebra", "Content"));
        list.add_item(create_test_content_item("Apple", "Content"));
        list.add_item(create_test_content_item("Banana", "Content"));
        
        list.sort_by_title();
        
        assert_eq!(list.items[0].title(), Some(&"Apple".to_string()));
        assert_eq!(list.items[1].title(), Some(&"Banana".to_string()));
        assert_eq!(list.items[2].title(), Some(&"Zebra".to_string()));
    }

    #[test]
    fn test_content_list_serialization() -> Result<(), Box<dyn std::error::Error>> {
        let mut list = ContentList::with_name("Serialization Test".to_string());
        list.add_item(create_test_content_item("Test Item", "Test Content"));
        
        // Test serialization
        let serialized = serde_json::to_string(&list)?;
        assert!(!serialized.is_empty());
        
        // Test deserialization
        let deserialized: ContentList = serde_json::from_str(&serialized)?;
        assert_eq!(list, deserialized);
        
        Ok(())
    }
}