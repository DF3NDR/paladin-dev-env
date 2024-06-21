use uuid::Uuid;
use url::Url;
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ContentItem {
    pub uuid: Uuid,
    pub created: DateTime<Utc>,
    pub modified: DateTime<Utc>,
    pub source_data: ContentSourceData,
    pub content: ContentType,
}

impl ContentItem {
    pub fn new(source_data: ContentSourceData, content: ContentType) -> Self {
        ContentItem {
            uuid: Uuid::new_v4(),
            created: Utc::now(),
            modified: Utc::now(),
            source_data,
            content,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ContentSourceData {
    pub id: ContentSourceId,
    pub url: Url,
    pub title: Option<String>,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
    pub source: Option<String>,
    pub author: Option<String>,
    pub pub_date: Option<DateTime<Utc>>,
    pub mod_date: Option<DateTime<Utc>>,
}

impl ContentSourceData {
    pub fn new(url: Url) -> Self {
        ContentSourceData {
            id: None,
            url,
            title: None,
            description: None,
            tags: None,
            source: None,
            author: None,
            pub_date: None,
            mod_date: None,
        }
    }
}

pub struct ContentSourceId {
    pub id: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[error("content source id already exists")]
pub struct ContentSourceIdDupeError;

impl ContentSourceId {
    pub fn new(id: String) -> Result<Self, ContentSourceIdDupeError> {
        if id.is_empty() {
            return Err(ContentSourceIdDupeError);
        }
        Ok(ContentSourceId { id })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ContentType {
    Text(TextContent),
    Video(VideoContent),
    Audio(AudioContent),
    Image(ImageContent),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TextContent {
    pub path: Option<String>,
    pub content: String,
}

impl TextContent {
    pub fn new(path: Option<String>, content: String) -> Self {
        TextContent { path, content }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VideoContent {
    pub path: Option<String>,
    pub duration: u64,  // Duration in seconds
}

impl VideoContent {
    pub fn new(path: Option<String>, duration: u64) -> Self {
        VideoContent { path, duration }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AudioContent {
    pub path: Option<String>,
    pub duration: u64,  // Duration in seconds
}

impl AudioContent {
    pub fn new(path: Option<String>, duration: u64) -> Self {
        AudioContent { path, duration }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImageContent {
    pub path: Option<String>,
    pub resolution: (u32, u32),  // Width and height
}

impl ImageContent {
    pub fn new(path: Option<String>, resolution: (u32, u32)) -> Self {
        ImageContent { path, resolution }
    }
}
