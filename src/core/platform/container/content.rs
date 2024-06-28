/*



*/

use uuid::Uuid;
use url::Url;
use chrono::{DateTime, Utc};
use std::hash::{Hasher, Hash};
use fasthash::{murmur3::Hash128_x64, FastHash};

use thiserror::Error;
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
    // Being a Domain Entity in Hexagonal Architecture the ContentItem should not have any knowledge of the repository
    pub fn new(
        content: ContentType, 
        // repository: &impl ContentRepository
    ) -> Result<Self, ContentItemError> {
        let mut content_item = ContentItem {
            uuid: Uuid::new_v4(),
            created: Utc::now(),
            modified: Utc::now(),
            content,
            url: None,
            hash: None,
            source_id: None,
            source_url: None,
            title: None,
            description: None,
            tags: None,
            source: None,
            author: None,
            pub_date: None,
            mod_date: None,
        };

        // content_item.update_hash(repository)?;

        Ok(content_item)
    }

    // Being a Domain Entity in Hexagonal Architecture the ContentItem should not have any knowledge of the repository
    pub fn update_content(
        &mut self, new_content: ContentType, 
        // repository: &impl ContentRepository
    ) -> Result<(), ContentItemError> {
        self.content = new_content;
        self.modified = Utc::now();
        // self.update_hash(repository)?;
        Ok(())
    }

    // Being a Domain Entity in Hexagonal Architecture the ContentItem should not have any knowledge of the repository
    fn update_hash(
        &mut self, 
        // repository: &impl ContentRepository
    ) -> Result<(), ContentItemError> {
        if let Some(path) = self.content.path() {
            let new_hash = generate_file_hash(&path)?;
            // if let Some(existing_content) = repository.get_by_hash(&new_hash)? {
            //     return Err(ContentItemError::HashAlreadyExists(existing_content.uuid));
            // }
            self.hash = Some(new_hash);
        } else {
            self.hash = None;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ContentType {
    Text(TextContent),
    Video(VideoContent),
    Audio(AudioContent),
    Image(ImageContent),
}

impl ContentType {
    pub fn path(&self) -> Option<&String> {
        match self {
            ContentType::Text(text) => text.path.as_ref(),
            ContentType::Video(video) => video.path.as_ref(),
            ContentType::Audio(audio) => audio.path.as_ref(),
            ContentType::Image(image) => image.path.as_ref(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TextContent {
    pub path: Option<String>,
    pub content: Option<String>,
    pub filesize: u64,
}

impl TextContent {
    pub fn new(path: Option<String>, content: Option<String>) -> Result<Self, ContentItemError> {
        let filesize = if let Some(ref path) = path {
            let metadata = std::fs::metadata(path)?;
            check_filesize(metadata.len())?;
            metadata.len()
        } else {
            0
        };
        Ok(TextContent { path, content, filesize })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VideoContent {
    pub path: Option<String>,
    pub duration: u64,
    pub filesize: u64,
}

impl VideoContent {
    pub fn new(path: Option<String>, duration: u64) -> Result<Self, ContentItemError> {
        let filesize = if let Some(ref path) = path {
            let metadata = std::fs::metadata(path)?;
            check_filesize(metadata.len())?;
            metadata.len()
        } else {
            0
        };
        Ok(VideoContent { path, duration, filesize })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AudioContent {
    pub path: Option<String>,
    pub duration: u64,
    pub filesize: u64,
}

impl AudioContent {
    pub fn new(path: Option<String>, duration: u64) -> Result<Self, ContentItemError> {
        let filesize = if let Some(ref path) = path {
            let metadata = std::fs::metadata(path)?;
            check_filesize(metadata.len())?;
            metadata.len()
        } else {
            0
        };
        Ok(AudioContent { path, duration, filesize })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ImageContent {
    pub path: Option<String>,
    pub resolution: (u32, u32),
    pub filesize: u64,
}

impl ImageContent {
    pub fn new(path: Option<String>, resolution: (u32, u32)) -> Result<Self, ContentItemError> {
        let filesize = if let Some(ref path) = path {
            let metadata = std::fs::metadata(path)?;
            check_filesize(metadata.len())?;
            metadata.len()
        } else {
            0
        };
        Ok(ImageContent { path, resolution, filesize })
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
    // not sure this will even work. Source for hash is supposed to be bytes not a Vec<u8>
    let hash = Hash128_x64::hash(buffer).to_string();
    Ok(format!("{:x}", hash))
}

fn check_filesize(filesize: u64) -> Result<(), ContentItemError> {
    let max_file_size = crate::config::application_settings::Settings::max_file_size();
    if filesize > max_file_size {
        Err(ContentItemError::FileSizeLimitExceeded)
    } else {
        Ok(())
    }
}
