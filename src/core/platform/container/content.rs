use serde::{Deserialize, Serialize};
use uuid::Uuid;
use url::Url;
use chrono::{DateTime, Utc};
use std::hash::{Hash, Hasher};
use std::io::Read;
use fasthash::{murmur3::Hash128_x64, FastHash};
use thiserror::Error;
use crate::core::base::entity::node::Node;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ContentItem {
    pub node: Node<ContentData>,
}

impl Hash for ContentItem {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.node.hash(state);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ContentData {
    pub content: ContentType,
    pub url: Option<Url>,
    pub hash: Option<String>,
    pub source_id: Option<String>,
    pub source_url: Option<Url>,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
    pub source: Option<String>,
    pub author: Option<String>,
    pub pub_date: Option<DateTime<Utc>>,
    pub mod_date: Option<DateTime<Utc>>,
}

impl ContentItem {
    pub fn new(content: ContentType) -> Result<Self, ContentItemError> {
        let hash = match content.path() {
            Some(path) => Some(generate_file_hash(path)?),
            None => None,
        };

        let content_data = ContentData {
            content,
            url: None,
            hash,
            source_id: None,
            source_url: None,
            description: None,
            tags: None,
            source: None,
            author: None,
            pub_date: None,
            mod_date: None,
        };

        let node = Node::new(content_data, None);

        Ok(ContentItem { node })
    }

    pub fn new_with_title(content: ContentType, title: String) -> Result<Self, ContentItemError> {
        let hash = match content.path() {
            Some(path) => Some(generate_file_hash(path)?),
            None => None,
        };

        let content_data = ContentData {
            content,
            url: None,
            hash,
            source_id: None,
            source_url: None,
            description: None,
            tags: None,
            source: None,
            author: None,
            pub_date: None,
            mod_date: None,
        };

        let node = Node::new(content_data, Some(title));

        Ok(ContentItem { node })
    }

    pub fn update_content(&mut self, new_content: ContentType) -> Result<(), ContentItemError> {
        // Update hash if content has a path
        let new_hash = match new_content.path() {
            Some(path) => Some(generate_file_hash(path)?),
            None => None,
        };

        // Update the content data
        self.node.node.content = new_content;
        self.node.node.hash = new_hash;
        self.node.node.mod_date = Some(Utc::now());
        
        // Update the node's modified timestamp
        self.node.modified = Utc::now();
        
        Ok(())
    }

    pub fn set_title(&mut self, title: Option<String>) {
        self.node.name = title;
        self.node.modified = Utc::now();
    }

    pub fn set_description(&mut self, description: Option<String>) {
        self.node.node.description = description;
        self.node.modified = Utc::now();
    }

    pub fn set_tags(&mut self, tags: Option<Vec<String>>) {
        self.node.node.tags = tags;
        self.node.modified = Utc::now();
    }

    pub fn set_author(&mut self, author: Option<String>) {
        self.node.node.author = author;
        self.node.modified = Utc::now();
    }

    pub fn set_source(&mut self, source: Option<String>) {
        self.node.node.source = source;
        self.node.modified = Utc::now();
    }

    pub fn set_url(&mut self, url: Option<Url>) {
        self.node.node.url = url;
        self.node.modified = Utc::now();
    }

    pub fn set_source_url(&mut self, source_url: Option<Url>) {
        self.node.node.source_url = source_url;
        self.node.modified = Utc::now();
    }
    
    pub fn set_source_id(&mut self, source_id: Option<String>) {
        self.node.node.source_id = source_id;
        self.node.modified = Utc::now();
    }

    pub fn set_publication_date(&mut self, pub_date: Option<DateTime<Utc>>) {
        self.node.node.pub_date = pub_date;
        self.node.modified = Utc::now();
    }

    pub fn disable_versioning(&mut self) {
        self.node.version = false;
    }

    pub fn enable_versioning(&mut self) {
        self.node.version = true;
    }

    // Convenience getters
    pub fn uuid(&self) -> Uuid {
        self.node.uuid
    }

    pub fn created(&self) -> DateTime<Utc> {
        self.node.created
    }

    pub fn modified(&self) -> DateTime<Utc> {
        self.node.modified
    }

    pub fn title(&self) -> Option<&String> {
        self.node.name.as_ref()
    }

    pub fn content(&self) -> &ContentType {
        &self.node.node.content
    }

    pub fn content_mut(&mut self) -> &mut ContentType {
        &mut self.node.node.content
    }

    pub fn description(&self) -> Option<&String> {
        self.node.node.description.as_ref()
    }

    pub fn tags(&self) -> Option<&Vec<String>> {
        self.node.node.tags.as_ref()
    }

    pub fn author(&self) -> Option<&String> {
        self.node.node.author.as_ref()
    }

    pub fn source(&self) -> Option<&String> {
        self.node.node.source.as_ref()
    }

    pub fn url(&self) -> Option<&Url> {
        self.node.node.url.as_ref()
    }

    pub fn source_url(&self) -> Option<&Url> {
        self.node.node.source_url.as_ref()
    }

    pub fn hash(&self) -> Option<&String> {
        self.node.node.hash.as_ref()
    }

    pub fn publication_date(&self) -> Option<DateTime<Utc>> {
        self.node.node.pub_date
    }

    pub fn modification_date(&self) -> Option<DateTime<Utc>> {
        self.node.node.mod_date
    }

    pub fn versioning_enabled(&self) -> bool {
        self.node.version
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ContentType {
    Text(TextContent),
    Video(VideoContent),
    Audio(AudioContent),
    Image(ImageContent),
}

impl ContentType {
    pub fn path(&self) -> Option<&String> {
        match self {
            ContentType::Text(content) => content.path.as_ref(),
            ContentType::Video(content) => content.path.as_ref(),
            ContentType::Audio(content) => content.path.as_ref(),
            ContentType::Image(content) => content.path.as_ref(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TextContent {
    pub path: Option<String>,
    pub content: Option<String>,
    pub filesize: u64,
}

impl TextContent {
    pub fn new(path: Option<String>, content: Option<String>) -> Result<Self, ContentItemError> {
        let filesize = match &path {
            Some(p) => {
                let metadata = std::fs::metadata(p).map_err(|_| ContentItemError::FileNotFound)?;
                let size = metadata.len();
                check_filesize(size)?;
                size
            }
            None => content.as_ref().map(|c| c.len() as u64).unwrap_or(0),
        };

        Ok(TextContent {
            path,
            content,
            filesize,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct VideoContent {
    pub path: Option<String>,
    pub duration: u64,
    pub filesize: u64,
}

impl VideoContent {
    pub fn new(path: Option<String>, duration: u64) -> Result<Self, ContentItemError> {
        let filesize = match &path {
            Some(p) => {
                let metadata = std::fs::metadata(p).map_err(|_| ContentItemError::FileNotFound)?;
                let size = metadata.len();
                check_filesize(size)?;
                size
            }
            None => 0,
        };

        Ok(VideoContent {
            path,
            duration,
            filesize,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct AudioContent {
    pub path: Option<String>,
    pub duration: u64,
    pub filesize: u64,
}

impl AudioContent {
    pub fn new(path: Option<String>, duration: u64) -> Result<Self, ContentItemError> {
        let filesize = match &path {
            Some(p) => {
                let metadata = std::fs::metadata(p).map_err(|_| ContentItemError::FileNotFound)?;
                let size = metadata.len();
                check_filesize(size)?;
                size
            }
            None => 0,
        };

        Ok(AudioContent {
            path,
            duration,
            filesize,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ImageContent {
    pub path: Option<String>,
    pub resolution: (u32, u32),
    pub filesize: u64,
}

impl ImageContent {
    pub fn new(path: Option<String>, resolution: (u32, u32)) -> Result<Self, ContentItemError> {
        let filesize = match &path {
            Some(p) => {
                let metadata = std::fs::metadata(p).map_err(|_| ContentItemError::FileNotFound)?;
                let size = metadata.len();
                check_filesize(size)?;
                size
            }
            None => 0,
        };

        Ok(ImageContent {
            path,
            resolution,
            filesize,
        })
    }
}

#[derive(Debug, Clone, Error)]
pub enum ContentItemError {
    #[error("File not found")]
    FileNotFound,
    #[error("Failed to read file")]
    FileReadError,
    #[error("File size exceeds the maximum limit")]
    FileSizeLimitExceeded,
    #[error("Content hash already exists in the system with UUID: {0}")]
    HashAlreadyExists(Uuid),
    #[error("No content item exists for that hash")]
    NoContentForHash,
}

fn generate_file_hash(path: &str) -> Result<String, ContentItemError> {
    let mut file = std::fs::File::open(path).map_err(|_| ContentItemError::FileNotFound)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).map_err(|_| ContentItemError::FileReadError)?;
    let hash = Hash128_x64::hash(&buffer);
    Ok(format!("{:x}", hash))
}

fn check_filesize(filesize: u64) -> Result<(), ContentItemError> {
    let max_file_size = 100 * 1024 * 1024; // 100MB
    if filesize > max_file_size {
        Err(ContentItemError::FileSizeLimitExceeded)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_item_new() {
        let text_content = TextContent::new(None, Some("test content".to_string())).unwrap();
        let content_type = ContentType::Text(text_content);
        
        let content_item = ContentItem::new(content_type.clone()).unwrap();
        
        assert_eq!(content_item.content(), &content_type);
        assert!(content_item.uuid() != Uuid::nil());
        assert!(content_item.created() <= Utc::now());
        assert_eq!(content_item.created(), content_item.modified());
        assert_eq!(content_item.url(), None);
        assert_eq!(content_item.hash(), None);
        assert!(content_item.versioning_enabled());
    }

    #[test]
    fn test_content_item_hash() {
        let text_content1 = TextContent::new(None, Some("content 1".to_string())).unwrap();
        let text_content2 = TextContent::new(None, Some("content 2".to_string())).unwrap();
        
        let item1 = ContentItem::new(ContentType::Text(text_content1)).unwrap();
        let item2 = ContentItem::new(ContentType::Text(text_content2)).unwrap();
        
        let mut hasher1 = std::collections::hash_map::DefaultHasher::new();
        let mut hasher2 = std::collections::hash_map::DefaultHasher::new();
        
        Hash::hash(&item1, &mut hasher1);
        Hash::hash(&item2, &mut hasher2);
        
        // Different content should produce different hashes
        assert_ne!(hasher1.finish(), hasher2.finish());
    }

    #[test]
    fn test_content_item_serialization() -> Result<(), Box<dyn std::error::Error>> {
        let text_content = TextContent::new(None, Some("serialize test".to_string()))?;
        let content_item = ContentItem::new_with_title(
            ContentType::Text(text_content),
            "Test Item".to_string(),
        )?;
        
        // Test serialization
        let serialized = serde_json::to_string(&content_item)?;
        assert!(!serialized.is_empty());
        
        // Test deserialization
        let deserialized: ContentItem = serde_json::from_str(&serialized)?;
        assert_eq!(content_item, deserialized);
        
        Ok(())
    }

    // ... rest of existing tests
}