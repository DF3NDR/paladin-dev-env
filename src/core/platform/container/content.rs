use serde::{Deserialize, Serialize};
/*
Content Item Domain Entity

ContentItem represents a piece of content in the system. It can be of various types such as text, video, audio, or image.
It contains metadata such as UUID, creation and modification timestamps, content type, URL, hash, source information, title, description, tags, author, and publication/modification dates.
It also provides methods to create a new content item, update its content, and generate a hash for the content.
ContentItem is a type of Node in the Hexagonal Architecture, meaning it is a core domain entity that should not have direct knowledge of the repository.


*/
use uuid::Uuid;
use url::Url;
use chrono::{DateTime, Utc};
use std::hash::Hash;
use std::io::Read;
use fasthash::{murmur3::Hash128_x64, FastHash};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ContentItem {
    pub uuid: Uuid,
    pub created: DateTime<Utc>,
    pub modified: DateTime<Utc>,
    pub content: ContentType,
    pub url: Option<Url>,
    pub hash: Option<String>,
    pub source_id: Option<String>,
    pub source_url: Option<Url>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
    pub source: Option<String>,
    pub author: Option<String>,
    pub pub_date: Option<DateTime<Utc>>,
    pub mod_date: Option<DateTime<Utc>>,
}

impl ContentItem {
    pub fn new(content: ContentType) -> Result<Self, ContentItemError> {
        let now = Utc::now();
        let hash = match content.path() {
            Some(path) => Some(generate_file_hash(path)?),
            None => None,
        };

        Ok(ContentItem {
            uuid: Uuid::new_v4(),
            created: now,
            modified: now,
            content,
            url: None,
            hash,
            source_id: None,
            source_url: None,
            title: None,
            description: None,
            tags: None,
            source: None,
            author: None,
            pub_date: None,
            mod_date: None,
        })
    }

    pub fn update_content(&mut self, new_content: ContentType) -> Result<(), ContentItemError> {
        self.content = new_content;
        self.modified = Utc::now();
        self.update_hash()?;
        Ok(())
    }

    fn update_hash(&mut self) -> Result<(), ContentItemError> {
        self.hash = match self.content.path() {
            Some(path) => Some(generate_file_hash(path)?),
            None => None,
        };
        Ok(())
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
            None => 0,
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
    // Since we don't have access to the settings module, let's use a reasonable default
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
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_content_item_new() {
        let text_content = TextContent::new(None, Some("test content".to_string())).unwrap();
        let content_type = ContentType::Text(text_content);
        
        let content_item = ContentItem::new(content_type.clone()).unwrap();
        
        assert_eq!(content_item.content, content_type);
        assert!(content_item.uuid != Uuid::nil());
        assert!(content_item.created <= Utc::now());
        assert_eq!(content_item.created, content_item.modified);
        assert_eq!(content_item.url, None);
        assert_eq!(content_item.hash, None);
    }

    #[test]
    fn test_content_item_update_content() {
        let initial_content = TextContent::new(None, Some("initial".to_string())).unwrap();
        let mut content_item = ContentItem::new(ContentType::Text(initial_content)).unwrap();
        
        let initial_modified = content_item.modified;
        std::thread::sleep(std::time::Duration::from_millis(1));
        
        let new_content = TextContent::new(None, Some("updated".to_string())).unwrap();
        content_item.update_content(ContentType::Text(new_content.clone())).unwrap();
        
        assert_eq!(content_item.content, ContentType::Text(new_content));
        assert!(content_item.modified > initial_modified);
    }

    #[test]
    fn test_text_content_new_with_content() {
        let text_content = TextContent::new(None, Some("test content".to_string())).unwrap();
        
        assert_eq!(text_content.path, None);
        assert_eq!(text_content.content, Some("test content".to_string()));
        assert_eq!(text_content.filesize, 0);
    }

    #[test]
    fn test_text_content_new_with_file() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempdir()?;
        let file_path = dir.path().join("test.txt");
        let mut file = File::create(&file_path)?;
        writeln!(file, "test file content")?;
        
        let path_str = file_path.to_string_lossy().to_string();
        let text_content = TextContent::new(Some(path_str.clone()), None)?;
        
        assert_eq!(text_content.path, Some(path_str));
        assert_eq!(text_content.content, None);
        assert!(text_content.filesize > 0);
        
        Ok(())
    }

    #[test]
    fn test_content_type_path() {
        let text_path = Some("text.txt".to_string());
        let text_content = TextContent::new(text_path.clone(), None).unwrap_or_else(|_| {
            // Fallback for when file doesn't exist in test
            TextContent { path: text_path.clone(), content: None, filesize: 0 }
        });
        let content_type = ContentType::Text(text_content);
        
        assert_eq!(content_type.path(), text_path.as_ref());
    }

    #[test]
    fn test_generate_file_hash() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempdir()?;
        let file_path = dir.path().join("hash_test.txt");
        let mut file = File::create(&file_path)?;
        writeln!(file, "content for hashing")?;
        
        let path_str = file_path.to_string_lossy().to_string();
        let hash = generate_file_hash(&path_str)?;
        
        assert!(!hash.is_empty());
        
        Ok(())
    }

    #[test]
    fn test_generate_file_hash_nonexistent_file() {
        let result = generate_file_hash("/nonexistent/file.txt");
        
        assert!(matches!(result, Err(ContentItemError::FileNotFound)));
    }

    #[test]
    fn test_all_content_types() -> Result<(), Box<dyn std::error::Error>> {
        // Test all content types can be created without files
        let text = TextContent::new(None, Some("text".to_string()))?;
        let video = VideoContent::new(None, 100)?;
        let audio = AudioContent::new(None, 200)?;
        let image = ImageContent::new(None, (800, 600))?;
        
        let _text_item = ContentItem::new(ContentType::Text(text))?;
        let _video_item = ContentItem::new(ContentType::Video(video))?;
        let _audio_item = ContentItem::new(ContentType::Audio(audio))?;
        let _image_item = ContentItem::new(ContentType::Image(image))?;
        
        Ok(())
    }
}